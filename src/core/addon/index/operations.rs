use std::collections::BTreeSet;
use std::path::PathBuf;

use super::matching::{
    explain_preflight_match_index_package_to_tracked_package,
    match_index_package_to_tracked_package, preflight_match_index_package_to_tracked_package,
};
use super::storage::{
    ensure_package_supports_flavor, find_index_package, load_addon_index,
    resolve_index_package_source,
};
use crate::core::addon::{
    AddonPackageMetadata, AddonProvider, AddonRegistry, AddonSourceRef, AddonStatePaths,
    DefaultAddonProvider, InstallAddonExecutionPlan, InstallPreparedAddonRequest,
    MissingDependencyCollectionRequest, MissingDependencyCollectionState,
    PreparePackageFromSourceRefTaskRequest, PreparePackageTaskContext, PreparedAddonPackage,
    TrackedAddonPackage, UpdatePreparedPackagesWithDependenciesRequest, UpdatedAddonPackageResult,
    collect_missing_dependency_prepared_packages, ensure_relink_addon_directories_match,
    execute_install_plan_task, list_addons, load_registry, no_tracked_packages_error,
    policy::AddonUpdatePolicySnapshot, prepare_install_prepared_addon,
    prepare_package_from_source_ref_task_with_provider, preview_installed_dependency_packages,
    provider::AddonSourceResolutionPolicy, relink_source_changed, relink_timestamp,
    rollback_or_report_addon_error, save_registry, select_single_tracked_package,
    update_prepared_packages_with_dependencies_task, validate_dependency_resolution_support,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressEvent,
    TaskProgressSink, emit_task_progress, ensure_task_not_cancelled,
};

use super::{
    AddonIndex, AddonIndexAttachPackageResult, AddonIndexAttachPackageStatus,
    AddonIndexAttachRequest, AddonIndexAttachResult, AddonIndexInstallRequest,
    AddonIndexInstallResult, AddonIndexPackage, AddonIndexRelinkRequest, AddonIndexRelinkResult,
    AddonIndexTrackedMatchStrategy, AddonIndexUpdateRequest, AddonIndexUpdateResult,
};

struct IndexInstallPlan {
    index_path: PathBuf,
    package: AddonIndexPackage,
    install_plan: InstallAddonExecutionPlan,
}

struct IndexAttachPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    index_path: PathBuf,
    index_name: String,
    dry_run: bool,
    registry_path: PathBuf,
    index_package_count: usize,
    considered_package_count: usize,
    skipped_unsupported_flavor_package_count: usize,
    packages: Vec<AddonIndexAttachPackageResult>,
    changes: Vec<IndexAttachChange>,
}

struct IndexAttachChange {
    package_result_index: usize,
    package: AddonIndexPackage,
    tracked_package_index: usize,
    tracked_package: TrackedAddonPackage,
    next_source: AddonSourceRef,
    metadata: AddonPackageMetadata,
    match_strategy: AddonIndexTrackedMatchStrategy,
    source_changed: bool,
    metadata_changed: bool,
}

struct IndexUpdatePlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    index_path: PathBuf,
    selected_packages: Vec<AddonIndexPackage>,
    registry: AddonRegistry,
    prepared_packages: Vec<PreparedAddonPackage>,
    dependency_prepared_packages: Vec<PreparedAddonPackage>,
    matched_packages: Vec<TrackedAddonPackage>,
    ignored_packages: Vec<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
    files_to_write: usize,
}

struct IndexRelinkPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    index_path: PathBuf,
    index_name: String,
    package: AddonIndexPackage,
    tracked_package_index: usize,
    tracked_package: TrackedAddonPackage,
    next_source: AddonSourceRef,
    metadata: AddonPackageMetadata,
    dry_run: bool,
    registry_path: PathBuf,
    source_changed: bool,
    metadata_changed: bool,
}

pub fn attach_addons_from_index(
    request: AddonIndexAttachRequest,
) -> AppResult<AddonIndexAttachResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    attach_addons_from_index_task(request, &cancellation, &mut progress)
}

pub fn attach_addons_from_index_task<TCancel, TProgress>(
    request: AddonIndexAttachRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexAttachResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    attach_addons_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn attach_addons_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexAttachRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexAttachResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexAttach,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index attach from `{}` for `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexAttach,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_attach_with_provider(provider, request, cancellation, progress)?;
    if plan.dry_run {
        let result = index_attach_result(plan, false);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexAttach,
            TaskPhase::Completed,
            format!(
                "Addon index attach dry run completed with {} planned change(s) and {} blocking package(s)",
                result.change_package_count, result.blocked_package_count
            ),
        );
        return Ok(result);
    }

    if !result_ready_for_attach(&plan) {
        let result = index_attach_result(plan, false);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexAttach,
            TaskPhase::Completed,
            format!(
                "Addon index attach blocked by {} package(s); no registry changes were written",
                result.blocked_package_count
            ),
        );
        return Ok(result);
    }

    if plan.changes.is_empty() {
        let result = index_attach_result(plan, false);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexAttach,
            TaskPhase::Completed,
            "Addon index attach found no registry changes to apply",
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonIndexAttach,
        TaskPhase::Executing,
        format!(
            "Attaching {} curated addon index package(s) without reinstalling live AddOns",
            plan.changes.len()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexAttach,
        TaskPhase::Executing,
    )?;

    let result = execute_index_attach_plan(plan)?;
    emit_task_progress(
        progress,
        TaskKind::AddonIndexAttach,
        TaskPhase::Completed,
        format!(
            "Addon index attach completed with {} attached package(s)",
            result.attached_package_count
        ),
    );
    Ok(result)
}

pub fn install_addon_from_index(
    request: AddonIndexInstallRequest,
) -> AppResult<AddonIndexInstallResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    install_addon_from_index_task(request, &cancellation, &mut progress)
}

pub fn install_addon_from_index_task<TCancel, TProgress>(
    request: AddonIndexInstallRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexInstallResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    install_addon_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn install_addon_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexInstallRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexInstallResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexInstall,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index install from `{}` into `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexInstall,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_install_with_provider(provider, request, cancellation, progress)?;
    let mut remapped_progress = RemappedTaskProgressSink {
        inner: progress,
        task: TaskKind::AddonIndexInstall,
    };
    let install =
        execute_install_plan_task(plan.install_plan, cancellation, &mut remapped_progress)
            .map_err(|error| {
                remap_cancelled_task_kind(
                    error,
                    TaskKind::AddonInstall,
                    TaskKind::AddonIndexInstall,
                )
            })?;

    Ok(AddonIndexInstallResult {
        index_path: plan.index_path,
        package: plan.package,
        install,
    })
}

pub fn relink_addon_from_index(
    request: AddonIndexRelinkRequest,
) -> AppResult<AddonIndexRelinkResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    relink_addon_from_index_task(request, &cancellation, &mut progress)
}

pub fn relink_addon_from_index_task<TCancel, TProgress>(
    request: AddonIndexRelinkRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexRelinkResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    relink_addon_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn relink_addon_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexRelinkRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexRelinkResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexRelink,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index relink from `{}` for `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexRelink,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_relink_with_provider(provider, request, cancellation, progress)?;
    if !plan.source_changed && !plan.metadata_changed {
        return Err(AppError::Validation(format!(
            "tracked addon package `{}` is already linked to index package `{}` from `{}`",
            plan.tracked_package.package_id, plan.package.id, plan.index_name
        )));
    }

    if plan.dry_run {
        let result = dry_run_index_relink_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexRelink,
            TaskPhase::Completed,
            format!(
                "Addon index relink dry run completed for `{}` (source changed: {}, metadata changed: {})",
                result.tracked_package_id, result.source_changed, result.metadata_changed
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonIndexRelink,
        TaskPhase::Executing,
        format!(
            "Relinking tracked package `{}` to index package `{}`",
            plan.tracked_package.package_id, plan.package.id
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexRelink,
        TaskPhase::Executing,
    )?;

    let result = execute_index_relink_plan(plan)?;
    emit_task_progress(
        progress,
        TaskKind::AddonIndexRelink,
        TaskPhase::Completed,
        format!(
            "Addon index relink completed for `{}` (source changed: {}, metadata changed: {})",
            result.tracked_package_id, result.source_changed, result.metadata_changed
        ),
    );
    Ok(result)
}

pub fn update_addons_from_index(
    request: AddonIndexUpdateRequest,
) -> AppResult<AddonIndexUpdateResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    update_addons_from_index_task(request, &cancellation, &mut progress)
}

pub fn update_addons_from_index_task<TCancel, TProgress>(
    request: AddonIndexUpdateRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexUpdateResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    update_addons_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn update_addons_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexUpdateRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexUpdateResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index update from `{}` for `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_update_with_provider(provider, request, cancellation, progress)?;
    if plan.selected_packages.is_empty() {
        let result = no_op_index_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexUpdate,
            TaskPhase::Completed,
            format!(
                "Addon index update completed without selected packages (ignored {} package(s))",
                result.update.ignored_packages.len()
            ),
        );
        return Ok(result);
    }
    if plan.dry_run {
        let result = dry_run_index_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexUpdate,
            TaskPhase::Completed,
            format!(
                "Addon index update dry run completed for {} package(s) with {} pending file(s)",
                result.selected_packages.len(),
                result.update.files_to_write
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::BackingUp,
        "Creating AddOns backup before addon index update",
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexUpdate,
        TaskPhase::BackingUp,
    )?;
    let backup_path =
        create_index_update_backup(&plan.installation, plan.backup_output_path.clone())?;

    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Executing,
        index_update_execution_message(
            plan.selected_packages.len(),
            plan.dependency_prepared_packages.len(),
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Executing,
    )?;
    let result = execute_index_update_plan(plan, backup_path, cancellation, progress)?;

    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Completed,
        format!(
            "Addon index update completed with {} written file(s)",
            result.update.written_files
        ),
    );
    Ok(result)
}

pub(crate) fn validate_addon_index_update_dependency_policy_support<P>(
    provider: &P,
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    index_path: &std::path::Path,
    name: Option<&str>,
) -> AppResult<()>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(index_path)?;
    let selected_packages = match name {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    for package in &selected_packages {
        ensure_package_supports_flavor(package, installation.flavor.as_str())?;
    }

    let inventory = list_addons(installation, state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Ok(());
    }

    let policies = AddonUpdatePolicySnapshot::load(installation, state_paths)?;
    let mut used_package_ids = BTreeSet::new();
    for package in &selected_packages {
        let Some(matched) = preflight_match_index_package_to_tracked_package(
            package,
            &inventory.tracked_packages,
            &used_package_ids,
        )?
        else {
            continue;
        };
        used_package_ids.insert(matched.package_id.clone());

        let package_policy = policies.index_update_policy(&matched);
        if name.is_none() && package_policy.ignored {
            continue;
        }
        if !package_policy.install_dependencies {
            continue;
        }

        let resolved_source = resolve_index_package_source(index_path, &package.source)?;
        let _ = validate_dependency_resolution_support(provider, &resolved_source)?;
    }

    Ok(())
}

fn preview_updated_packages(
    matched_packages: &[TrackedAddonPackage],
    prepared_packages: &[PreparedAddonPackage],
) -> Vec<TrackedAddonPackage> {
    matched_packages
        .iter()
        .zip(prepared_packages.iter())
        .map(|(matched, prepared)| TrackedAddonPackage {
            package_id: prepared.package_id.clone(),
            source: prepared.source.clone(),
            installed_at: matched.installed_at.clone(),
            updated_at: String::new(),
            addons: prepared
                .addons
                .iter()
                .map(|addon| addon.addon.clone())
                .collect(),
            metadata: prepared
                .metadata
                .clone()
                .or_else(|| matched.metadata.clone()),
        })
        .collect()
}

pub(super) fn metadata_from_index_package(
    index: &AddonIndex,
    package: &AddonIndexPackage,
) -> AddonPackageMetadata {
    AddonPackageMetadata {
        index_name: Some(index.name.clone()),
        index_package_id: Some(package.id.clone()),
        package_name: Some(package.name.clone()),
        version: Some(package.version.clone()),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.sha256.clone(),
        supported_flavors: package.supported_flavors.clone(),
    }
}

fn prepare_index_attach_with_provider<P>(
    provider: &P,
    request: AddonIndexAttachRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexAttachPlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let inventory = list_addons(&request.installation, &request.state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let registry = load_registry(&request.installation, &request.state_paths)?;
    let flavor = request.installation.flavor.as_str();
    let selected_packages = match request.name.as_deref() {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    let mut packages = Vec::new();
    let mut changes = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    let mut considered_package_count = 0usize;
    let mut skipped_unsupported_flavor_package_count = 0usize;

    for package in selected_packages {
        if let Err(error) = ensure_package_supports_flavor(&package, flavor) {
            if request.name.is_some() {
                return Err(error);
            }
            skipped_unsupported_flavor_package_count += 1;
            packages.push(skipped_unsupported_flavor_attach_result(
                package,
                error.to_string(),
            ));
            continue;
        }

        considered_package_count += 1;
        let package_for_matching =
            resolved_index_package_for_matching(&request.index_path, &package);
        let explained = match explain_preflight_match_index_package_to_tracked_package(
            &package_for_matching,
            &inventory.tracked_packages,
            &used_package_ids,
        ) {
            Ok(Some(matched)) => matched,
            Ok(None) => {
                packages.push(no_local_match_attach_result(package));
                continue;
            }
            Err(error) => {
                packages.push(ambiguous_local_match_attach_result(
                    package,
                    error.to_string(),
                ));
                continue;
            }
        };
        used_package_ids.insert(explained.package.package_id.clone());
        let tracked_package = explained.package;
        let match_strategy = explained.strategy;

        let tracked_package_index = registry
            .packages
            .iter()
            .position(|candidate| *candidate == tracked_package)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could plan",
                    tracked_package.package_id
                ))
            })?;
        let resolved_source =
            match resolve_index_package_source(&request.index_path, &package.source) {
                Ok(source) => source,
                Err(error) => {
                    packages.push(prepare_failed_attach_result(
                        package,
                        Some(tracked_package.package_id.as_str()),
                        Some(match_strategy.clone()),
                        Some(tracked_package.source.clone()),
                        None,
                        error.to_string(),
                    ));
                    continue;
                }
            };
        let prepared = match prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &resolved_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    TaskKind::AddonIndexAttach,
                    TaskPhase::Preparing,
                ),
            ),
            progress,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                packages.push(prepare_failed_attach_result(
                    package,
                    Some(tracked_package.package_id.as_str()),
                    Some(match_strategy.clone()),
                    Some(tracked_package.source.clone()),
                    Some(resolved_source),
                    error.to_string(),
                ));
                continue;
            }
        };
        if let Err(error) = ensure_relink_addon_directories_match(&tracked_package, &prepared) {
            packages.push(addon_directory_mismatch_attach_result(
                package,
                &tracked_package.package_id,
                match_strategy.clone(),
                tracked_package.source.clone(),
                prepared.source.clone(),
                error.to_string(),
            ));
            continue;
        }

        let metadata = metadata_from_index_package(&index, &package);
        let source_changed = relink_source_changed(&tracked_package, &prepared.source);
        let metadata_changed = tracked_package.metadata.as_ref() != Some(&metadata);
        if !source_changed && !metadata_changed {
            packages.push(already_attached_attach_result(
                package,
                &tracked_package.package_id,
                match_strategy,
                tracked_package.source.clone(),
            ));
            continue;
        }

        let package_result_index = packages.len();
        packages.push(ready_attach_result(ReadyAttachResultRequest {
            package: package.clone(),
            tracked_package_id: &tracked_package.package_id,
            match_strategy: match_strategy.clone(),
            previous_source: tracked_package.source.clone(),
            source: prepared.source.clone(),
            source_changed,
            metadata_changed,
            applied: false,
        }));
        changes.push(IndexAttachChange {
            package_result_index,
            package,
            tracked_package_index,
            tracked_package,
            next_source: prepared.source,
            metadata,
            match_strategy,
            source_changed,
            metadata_changed,
        });
    }

    Ok(IndexAttachPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        index_path: request.index_path,
        index_name: index.name,
        dry_run: request.dry_run,
        registry_path: inventory.registry_path,
        index_package_count: index.packages.len(),
        considered_package_count,
        skipped_unsupported_flavor_package_count,
        packages,
        changes,
    })
}

fn prepare_index_relink_with_provider<P>(
    provider: &P,
    request: AddonIndexRelinkRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexRelinkPlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let package = find_index_package(&index, &request.name)?.clone();
    ensure_package_supports_flavor(&package, request.installation.flavor.as_str())?;

    let inventory = list_addons(&request.installation, &request.state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let registry = load_registry(&request.installation, &request.state_paths)?;
    let resolved_source = resolve_index_package_source(&request.index_path, &package.source)?;
    let prepared = prepare_package_from_source_ref_task_with_provider(
        provider,
        PreparePackageFromSourceRefTaskRequest::new(
            &resolved_source,
            PreparePackageTaskContext::new(
                Some(request.installation.flavor),
                request.installation.platform,
                cancellation,
                TaskKind::AddonIndexRelink,
                TaskPhase::Preparing,
            ),
        ),
        progress,
    )?;

    let (tracked_package_index, tracked_package) = match request.target.as_deref() {
        Some(target) if target.trim().is_empty() => {
            return Err(AppError::Validation(
                "tracked addon selector for addon index relink must not be empty".to_string(),
            ));
        }
        Some(target) => select_single_tracked_package(&registry, target.trim())?,
        None => {
            let matched = match_index_package_to_tracked_package(
                &package,
                &prepared,
                &inventory.tracked_packages,
                &BTreeSet::new(),
            )?;
            let tracked_package_index = registry
                .packages
                .iter()
                .position(|candidate| *candidate == matched)
                .ok_or_else(|| {
                    AppError::Validation(format!(
                        "tracked addon package `{}` disappeared before addon index relink",
                        matched.package_id
                    ))
                })?;
            (tracked_package_index, matched)
        }
    };
    ensure_relink_addon_directories_match(&tracked_package, &prepared)?;

    let metadata = metadata_from_index_package(&index, &package);
    let source_changed = relink_source_changed(&tracked_package, &prepared.source);
    let metadata_changed = tracked_package.metadata.as_ref() != Some(&metadata);

    Ok(IndexRelinkPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        index_path: request.index_path,
        index_name: index.name,
        package,
        tracked_package_index,
        tracked_package,
        next_source: prepared.source,
        metadata,
        dry_run: request.dry_run,
        registry_path: inventory.registry_path,
        source_changed,
        metadata_changed,
    })
}

fn resolved_index_package_for_matching(
    index_path: &std::path::Path,
    package: &AddonIndexPackage,
) -> AddonIndexPackage {
    let mut resolved = package.clone();
    if let Ok(source) = resolve_index_package_source(index_path, &package.source) {
        resolved.source = source;
    }
    resolved
}

fn prepare_index_install_with_provider<P>(
    provider: &P,
    request: AddonIndexInstallRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexInstallPlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let package = find_index_package(&index, &request.name)?.clone();
    ensure_package_supports_flavor(&package, request.installation.flavor.as_str())?;
    let resolved_source = resolve_index_package_source(&request.index_path, &package.source)?;
    let prepared = prepare_package_from_source_ref_task_with_provider(
        provider,
        PreparePackageFromSourceRefTaskRequest::new(
            &resolved_source,
            PreparePackageTaskContext::new(
                Some(request.installation.flavor),
                request.installation.platform,
                cancellation,
                TaskKind::AddonIndexInstall,
                TaskPhase::Preparing,
            ),
        ),
        progress,
    )?;
    let install_plan = prepare_install_prepared_addon(InstallPreparedAddonRequest {
        installation: request.installation,
        state_paths: request.state_paths,
        prepared,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        metadata: Some(metadata_from_index_package(&index, &package)),
    })?;

    Ok(IndexInstallPlan {
        index_path: request.index_path,
        package: package.clone(),
        install_plan,
    })
}

fn dry_run_index_relink_result(plan: IndexRelinkPlan) -> AddonIndexRelinkResult {
    AddonIndexRelinkResult {
        index_path: plan.index_path,
        package: plan.package,
        dry_run: true,
        tracked_package_id: plan.tracked_package.package_id,
        previous_source: plan.tracked_package.source,
        source: plan.next_source,
        addons: plan.tracked_package.addons,
        metadata: plan.metadata,
        registry_path: plan.registry_path,
        source_changed: plan.source_changed,
        metadata_changed: plan.metadata_changed,
    }
}

fn execute_index_attach_plan(plan: IndexAttachPlan) -> AppResult<AddonIndexAttachResult> {
    let mut registry = load_registry(&plan.installation, &plan.state_paths)?;
    let timestamp = relink_timestamp()?;

    for change in &plan.changes {
        let target = registry
            .packages
            .get(change.tracked_package_index)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could be applied",
                    change.tracked_package.package_id
                ))
            })?;
        if *target != change.tracked_package {
            return Err(AppError::Validation(format!(
                "tracked addon package `{}` changed before addon index attach could be applied",
                change.tracked_package.package_id
            )));
        }
    }

    for change in &plan.changes {
        let target = registry
            .packages
            .get_mut(change.tracked_package_index)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could be applied",
                    change.tracked_package.package_id
                ))
            })?;
        target.source = change.next_source.clone();
        target.updated_at = timestamp.clone();
        target.metadata = Some(change.metadata.clone());
    }
    save_registry(&plan.installation, &plan.state_paths, &registry)?;

    Ok(index_attach_result(plan, true))
}

fn index_attach_result(mut plan: IndexAttachPlan, applied: bool) -> AddonIndexAttachResult {
    if applied {
        for change in &plan.changes {
            plan.packages[change.package_result_index] =
                ready_attach_result(ReadyAttachResultRequest {
                    package: change.package.clone(),
                    tracked_package_id: &change.tracked_package.package_id,
                    match_strategy: change.match_strategy.clone(),
                    previous_source: change.tracked_package.source.clone(),
                    source: change.next_source.clone(),
                    source_changed: change.source_changed,
                    metadata_changed: change.metadata_changed,
                    applied: true,
                });
        }
    }

    let change_package_count = plan.changes.len();
    let attached_package_count = if applied { change_package_count } else { 0 };
    let already_attached_package_count = plan
        .packages
        .iter()
        .filter(|package| {
            matches!(
                package.status,
                AddonIndexAttachPackageStatus::AlreadyAttached
            )
        })
        .count();
    let blocked_package_count = attach_blocked_package_count(&plan.packages);

    AddonIndexAttachResult {
        index_path: plan.index_path,
        index_name: plan.index_name,
        dry_run: plan.dry_run,
        ready: blocked_package_count == 0,
        applied,
        registry_path: plan.registry_path,
        index_package_count: plan.index_package_count,
        considered_package_count: plan.considered_package_count,
        change_package_count,
        attached_package_count,
        already_attached_package_count,
        blocked_package_count,
        skipped_unsupported_flavor_package_count: plan.skipped_unsupported_flavor_package_count,
        packages: plan.packages,
    }
}

fn result_ready_for_attach(plan: &IndexAttachPlan) -> bool {
    attach_blocked_package_count(&plan.packages) == 0
}

fn attach_blocked_package_count(packages: &[AddonIndexAttachPackageResult]) -> usize {
    packages
        .iter()
        .filter(|package| attach_status_is_blocking(&package.status))
        .count()
}

fn attach_status_is_blocking(status: &AddonIndexAttachPackageStatus) -> bool {
    matches!(
        status,
        AddonIndexAttachPackageStatus::NoLocalMatch
            | AddonIndexAttachPackageStatus::AmbiguousLocalMatch
            | AddonIndexAttachPackageStatus::AddonDirectoryMismatch
            | AddonIndexAttachPackageStatus::PrepareFailed
    )
}

struct ReadyAttachResultRequest<'a> {
    package: AddonIndexPackage,
    tracked_package_id: &'a str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    previous_source: AddonSourceRef,
    source: AddonSourceRef,
    source_changed: bool,
    metadata_changed: bool,
    applied: bool,
}

fn ready_attach_result(request: ReadyAttachResultRequest<'_>) -> AddonIndexAttachPackageResult {
    let ReadyAttachResultRequest {
        package,
        tracked_package_id,
        match_strategy,
        previous_source,
        source,
        source_changed,
        metadata_changed,
        applied,
    } = request;
    let status = if applied {
        AddonIndexAttachPackageStatus::Attached
    } else {
        AddonIndexAttachPackageStatus::WouldAttach
    };

    AddonIndexAttachPackageResult {
        package,
        status,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy.clone()),
        previous_source: Some(previous_source),
        source: Some(source),
        source_changed,
        metadata_changed,
        message: attach_change_message(
            tracked_package_id,
            &match_strategy,
            source_changed,
            metadata_changed,
            applied,
        ),
    }
}

fn already_attached_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: &str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    source: AddonSourceRef,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AlreadyAttached,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy.clone()),
        previous_source: Some(source.clone()),
        source: Some(source),
        source_changed: false,
        metadata_changed: false,
        message: format!(
            "matched tracked package `{}` by {}; source and curated metadata already match",
            tracked_package_id,
            match_strategy_label(&match_strategy)
        ),
    }
}

fn no_local_match_attach_result(package: AddonIndexPackage) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::NoLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message: "no tracked addon package from the current registry matched this index package"
            .to_string(),
    }
}

fn ambiguous_local_match_attach_result(
    package: AddonIndexPackage,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AmbiguousLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn addon_directory_mismatch_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: &str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    previous_source: AddonSourceRef,
    source: AddonSourceRef,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AddonDirectoryMismatch,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy),
        previous_source: Some(previous_source),
        source: Some(source),
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn prepare_failed_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: Option<&str>,
    match_strategy: Option<AddonIndexTrackedMatchStrategy>,
    previous_source: Option<AddonSourceRef>,
    source: Option<AddonSourceRef>,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::PrepareFailed,
        matched_tracked_package_id: tracked_package_id.map(|value| value.to_string()),
        match_strategy,
        previous_source,
        source,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn skipped_unsupported_flavor_attach_result(
    package: AddonIndexPackage,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::SkippedUnsupportedFlavor,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn attach_change_message(
    tracked_package_id: &str,
    strategy: &AddonIndexTrackedMatchStrategy,
    source_changed: bool,
    metadata_changed: bool,
    applied: bool,
) -> String {
    let action = match (source_changed, metadata_changed, applied) {
        (true, true, true) => "attached curated source and metadata",
        (true, true, false) => "would attach curated source and metadata",
        (true, false, true) => "relinked the tracked source to the curated source",
        (true, false, false) => "would relink the tracked source to the curated source",
        (false, true, true) => "attached curated metadata",
        (false, true, false) => "would attach curated metadata",
        (false, false, true) => "left the tracked package unchanged",
        (false, false, false) => "would leave the tracked package unchanged",
    };

    format!(
        "matched tracked package `{}` by {}; {} without reinstalling live AddOns",
        tracked_package_id,
        match_strategy_label(strategy),
        action
    )
}

fn match_strategy_label(strategy: &AddonIndexTrackedMatchStrategy) -> &'static str {
    match strategy {
        AddonIndexTrackedMatchStrategy::StoredIndexPackageId => "stored index package id",
        AddonIndexTrackedMatchStrategy::ExactPackageId => "exact package id",
        AddonIndexTrackedMatchStrategy::CuratedMatchPackageId => "curated match_package_ids hint",
        AddonIndexTrackedMatchStrategy::SourceIdentity => "source identity",
        AddonIndexTrackedMatchStrategy::SourceFamilyIdentity => "source family identity",
        AddonIndexTrackedMatchStrategy::DisplayName => "display name",
        AddonIndexTrackedMatchStrategy::AddonDirectories => "addon directories",
        AddonIndexTrackedMatchStrategy::AddonDirectoryOverlap => "addon directory overlap",
    }
}

fn prepare_index_update_with_provider<P>(
    provider: &P,
    request: AddonIndexUpdateRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexUpdatePlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let selected_packages = match &request.name {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    for package in &selected_packages {
        ensure_package_supports_flavor(package, request.installation.flavor.as_str())?;
    }

    let inventory = list_addons(&request.installation, &request.state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let registry = load_registry(&request.installation, &request.state_paths)?;
    let policies = AddonUpdatePolicySnapshot::load(&request.installation, &request.state_paths)?;
    let mut prepared_packages = Vec::new();
    let mut dependency_prepared_packages = Vec::new();
    let mut matched_packages = Vec::new();
    let mut effective_selected_packages = Vec::new();
    let mut ignored_packages = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    let mut planned_dependency_keys = BTreeSet::new();
    for package in &selected_packages {
        let preflight_matched = preflight_match_index_package_to_tracked_package(
            package,
            &inventory.tracked_packages,
            &used_package_ids,
        )?;
        if let Some(matched) = preflight_matched.as_ref() {
            let package_policy = policies.index_update_policy(matched);
            if request.name.is_none() && package_policy.ignored {
                used_package_ids.insert(matched.package_id.clone());
                ignored_packages.push(matched.package_id.clone());
                continue;
            }
        }

        let resolved_source = resolve_index_package_source(&request.index_path, &package.source)?;
        let mut prepared = prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &resolved_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    TaskKind::AddonIndexUpdate,
                    TaskPhase::Preparing,
                ),
            ),
            progress,
        )?;
        prepared.metadata = Some(metadata_from_index_package(&index, package));
        let matched = match_index_package_to_tracked_package(
            package,
            &prepared,
            &inventory.tracked_packages,
            &used_package_ids,
        )?;
        used_package_ids.insert(matched.package_id.clone());
        let package_policy = policies.index_update_policy(&matched);
        if request.name.is_none() && package_policy.ignored {
            ignored_packages.push(matched.package_id.clone());
            continue;
        }
        prepared.package_id = matched.package_id.clone();
        if package_policy.install_dependencies {
            if preflight_matched.is_none() {
                let _ = validate_dependency_resolution_support(provider, &resolved_source)
                    .map_err(|error| {
                        annotate_deferred_dependency_support_error(
                            error,
                            package,
                            &matched,
                            &resolved_source,
                        )
                    })?;
            }
            collect_missing_dependency_prepared_packages(
                provider,
                MissingDependencyCollectionRequest {
                    source: &resolved_source,
                    resolution_policy: AddonSourceResolutionPolicy::default(),
                    installation: &request.installation,
                    registry: &registry,
                    selected_packages: &matched_packages,
                    task_kind: TaskKind::AddonIndexUpdate,
                },
                &mut MissingDependencyCollectionState {
                    prepared_packages: &mut dependency_prepared_packages,
                    planned_keys: &mut planned_dependency_keys,
                },
                cancellation,
                progress,
            )?;
        }
        effective_selected_packages.push(package.clone());
        prepared_packages.push(prepared);
        matched_packages.push(matched);
    }
    ignored_packages.sort();

    let files_to_write = prepared_packages
        .iter()
        .chain(dependency_prepared_packages.iter())
        .map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.file_count)
                .sum::<usize>()
        })
        .sum::<usize>();

    Ok(IndexUpdatePlan {
        installation: request.installation,
        state_paths: request.state_paths,
        index_path: request.index_path,
        selected_packages: effective_selected_packages,
        registry,
        prepared_packages,
        dependency_prepared_packages,
        matched_packages,
        ignored_packages,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        registry_path: inventory.registry_path,
        files_to_write,
    })
}

fn no_op_index_update_result(plan: IndexUpdatePlan) -> AddonIndexUpdateResult {
    let IndexUpdatePlan {
        index_path,
        selected_packages,
        registry_path,
        ignored_packages,
        dry_run,
        ..
    } = plan;

    AddonIndexUpdateResult {
        index_path,
        selected_packages,
        update: UpdatedAddonPackageResult {
            dry_run,
            registry_path,
            files_to_write: 0,
            written_files: 0,
            updated_packages: Vec::new(),
            installed_dependency_packages: Vec::new(),
            ignored_packages,
            backup_path: None,
        },
    }
}

fn execute_index_relink_plan(plan: IndexRelinkPlan) -> AppResult<AddonIndexRelinkResult> {
    let IndexRelinkPlan {
        installation,
        state_paths,
        index_path,
        package,
        tracked_package_index,
        tracked_package,
        next_source,
        metadata,
        registry_path,
        source_changed,
        metadata_changed,
        ..
    } = plan;

    let mut registry = load_registry(&installation, &state_paths)?;
    let target = registry
        .packages
        .get_mut(tracked_package_index)
        .ok_or_else(|| {
            AppError::Validation(format!(
                "tracked addon package `{}` disappeared before addon index relink",
                tracked_package.package_id
            ))
        })?;

    if *target != tracked_package {
        return Err(AppError::Validation(format!(
            "tracked addon package `{}` changed before addon index relink could be applied",
            tracked_package.package_id
        )));
    }

    target.source = next_source.clone();
    target.updated_at = relink_timestamp()?;
    target.metadata = Some(metadata.clone());
    save_registry(&installation, &state_paths, &registry)?;

    Ok(AddonIndexRelinkResult {
        index_path,
        package,
        dry_run: false,
        tracked_package_id: tracked_package.package_id,
        previous_source: tracked_package.source,
        source: next_source,
        addons: tracked_package.addons,
        metadata,
        registry_path,
        source_changed,
        metadata_changed,
    })
}

fn dry_run_index_update_result(plan: IndexUpdatePlan) -> AddonIndexUpdateResult {
    let IndexUpdatePlan {
        index_path,
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
        matched_packages,
        ignored_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    AddonIndexUpdateResult {
        index_path,
        selected_packages,
        update: UpdatedAddonPackageResult {
            dry_run: true,
            registry_path,
            files_to_write,
            written_files: 0,
            updated_packages: preview_updated_packages(&matched_packages, &prepared_packages),
            installed_dependency_packages: preview_installed_dependency_packages(
                &dependency_prepared_packages,
            ),
            ignored_packages,
            backup_path: None,
        },
    }
}

fn execute_index_update_plan<TCancel, TProgress>(
    plan: IndexUpdatePlan,
    backup_path: PathBuf,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexUpdateResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let IndexUpdatePlan {
        installation,
        state_paths,
        index_path,
        selected_packages,
        registry,
        prepared_packages,
        dependency_prepared_packages,
        matched_packages,
        ignored_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    match update_prepared_packages_with_dependencies_task(
        &installation,
        &state_paths,
        UpdatePreparedPackagesWithDependenciesRequest {
            registry,
            selected_packages: matched_packages,
            prepared_packages,
            dependency_prepared_packages,
            task: TaskKind::AddonIndexUpdate,
        },
        cancellation,
        progress,
    ) {
        Ok((updated_packages, installed_dependency_packages, written_files)) => {
            Ok(AddonIndexUpdateResult {
                index_path,
                selected_packages,
                update: UpdatedAddonPackageResult {
                    dry_run: false,
                    registry_path,
                    files_to_write,
                    written_files,
                    updated_packages,
                    installed_dependency_packages,
                    ignored_packages,
                    backup_path: Some(backup_path),
                },
            })
        }
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}

fn create_index_update_backup(
    installation: &DetectedFlavorInstallation,
    output_path: Option<PathBuf>,
) -> AppResult<PathBuf> {
    Ok(create_backup(BackupRequest {
        installation: installation.clone(),
        output_path,
        groups: vec![BackupGroup::Addons],
        label: Some("addon-index-update".to_string()),
    })?
    .archive_path)
}

fn index_update_execution_message(updated_count: usize, dependency_count: usize) -> String {
    match dependency_count {
        0 => format!("Updating {updated_count} addon index package(s)"),
        _ => format!(
            "Updating {updated_count} addon index package(s) and installing {dependency_count} dependency package(s)"
        ),
    }
}

fn annotate_deferred_dependency_support_error(
    error: AppError,
    package: &AddonIndexPackage,
    matched: &TrackedAddonPackage,
    source: &AddonSourceRef,
) -> AppError {
    match error {
        AppError::Validation(message) => AppError::Validation(format!(
            "{message} while updating addon index package `{}` matched to tracked package `{}`. addon-index app preflight could not determine this mapping from stable index identity hints alone, so the unsupported dependency policy surfaced only during domain update preparation. Consider adding exact `match_package_ids`, stable `addon_directories`, or unique exact package-name continuity for source `{}`.",
            package.id,
            matched.package_id,
            source.display_name()
        )),
        other => other,
    }
}

fn remap_cancelled_task_kind(error: AppError, from_task: TaskKind, to_task: TaskKind) -> AppError {
    match error {
        AppError::Cancelled(message) => {
            AppError::Cancelled(message.replace(from_task.as_str(), to_task.as_str()))
        }
        other => other,
    }
}

struct RemappedTaskProgressSink<'a, TProgress> {
    inner: &'a mut TProgress,
    task: TaskKind,
}

impl<TProgress> TaskProgressSink for RemappedTaskProgressSink<'_, TProgress>
where
    TProgress: TaskProgressSink,
{
    fn push(&mut self, mut event: TaskProgressEvent) {
        event.task = self.task;
        self.inner.push(event);
    }

    fn task_id(&self) -> Option<&str> {
        self.inner.task_id()
    }
}
