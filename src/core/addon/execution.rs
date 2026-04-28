use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};

use super::find_existing_addon_path;
use super::registry::registry_path;
use super::{
    AddonPackageMetadata, AddonProvider, AddonRegistry, AddonStatePaths, DefaultAddonProvider,
    InstallAddonRequest, InstalledAddonPackageResult, MissingDependencyCollectionRequest,
    MissingDependencyCollectionState, PreparePackageFromSourceInputTaskRequest,
    PreparePackageFromSourceRefTaskRequest, PreparePackageTaskContext, PreparedAddonPackage,
    RemoveAddonRequest, RemovedAddonPackageResult, TrackedAddonPackage, UpdateAddonRequest,
    UpdatePreparedPackagesWithDependenciesRequest, UpdatedAddonPackageResult,
    collect_missing_dependency_prepared_packages, install_prepared_package_task, load_registry,
    no_tracked_packages_error, policy::AddonUpdatePolicySnapshot,
    prepare_package_from_source_input_task_with_provider,
    prepare_package_from_source_ref_task_with_provider, preview_installed_dependency_packages,
    remove_selected_packages_task, rollback_or_report_addon_error, select_tracked_packages,
    update_prepared_packages_with_dependencies_task,
};

#[derive(Debug)]
pub(crate) struct InstallPreparedAddonRequest {
    pub(crate) installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub(crate) prepared: PreparedAddonPackage,
    pub(crate) dry_run: bool,
    pub(crate) backup_output_path: Option<PathBuf>,
    pub(crate) replace_existing: bool,
    pub(crate) metadata: Option<AddonPackageMetadata>,
}

pub(crate) struct InstallAddonExecutionPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    prepared: PreparedAddonPackage,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    replace_existing: bool,
    registry_path: PathBuf,
    files_to_write: usize,
    replaced_addons: Vec<String>,
}

struct UpdateAddonsExecutionPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    registry: AddonRegistry,
    selected_packages: Vec<TrackedAddonPackage>,
    prepared_packages: Vec<PreparedAddonPackage>,
    dependency_prepared_packages: Vec<PreparedAddonPackage>,
    ignored_packages: Vec<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
    files_to_write: usize,
}

struct RemoveAddonsExecutionPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    removed_packages: Vec<TrackedAddonPackage>,
    removed_addons: Vec<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
}

pub fn install_addon(request: InstallAddonRequest) -> AppResult<InstalledAddonPackageResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    install_addon_task(request, &cancellation, &mut progress)
}

pub fn install_addon_task<TCancel, TProgress>(
    request: InstallAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    install_addon_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn install_addon_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: InstallAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::Preparing,
        format!(
            "Preparing addon installation from `{}` into `{}`",
            request.source,
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonInstall, TaskPhase::Preparing)?;

    let plan = prepare_install_addon_with_provider(provider, request, cancellation, progress)?;
    execute_install_plan_task(plan, cancellation, progress)
}

pub(crate) fn execute_install_plan_task<TCancel, TProgress>(
    plan: InstallAddonExecutionPlan,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    if plan.dry_run {
        let result = dry_run_install_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonInstall,
            TaskPhase::Completed,
            format!(
                "Addon install dry run completed with {} addon(s) and {} pending file(s)",
                result.addons.len(),
                result.files_to_write
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::BackingUp,
        "Creating AddOns backup before addon install",
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonInstall, TaskPhase::BackingUp)?;
    let backup_path = create_addon_backup(
        &plan.installation,
        plan.backup_output_path.clone(),
        "addon-install",
    )?;

    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::Executing,
        format!(
            "Installing {} addon directory(s)",
            plan.prepared.addons.len()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonInstall, TaskPhase::Executing)?;

    let result = execute_install_plan(plan, backup_path, cancellation, progress)?;
    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::Completed,
        format!(
            "Addon install completed with {} written file(s)",
            result.written_files
        ),
    );
    Ok(result)
}

pub fn update_addons(request: UpdateAddonRequest) -> AppResult<UpdatedAddonPackageResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    update_addons_task(request, &cancellation, &mut progress)
}

pub fn update_addons_task<TCancel, TProgress>(
    request: UpdateAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UpdatedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    update_addons_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn update_addons_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: UpdateAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UpdatedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::Preparing,
        format!(
            "Preparing addon update for `{}`",
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonUpdate, TaskPhase::Preparing)?;

    let plan = prepare_update_addons_with_provider(provider, request, cancellation, progress)?;
    if plan.selected_packages.is_empty() {
        let result = no_op_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonUpdate,
            TaskPhase::Completed,
            format!(
                "Addon update completed without selected packages (ignored {} package(s))",
                result.ignored_packages.len()
            ),
        );
        return Ok(result);
    }
    if plan.dry_run {
        let result = dry_run_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonUpdate,
            TaskPhase::Completed,
            format!(
                "Addon update dry run completed with {} package(s) and {} pending file(s)",
                result.updated_packages.len(),
                result.files_to_write
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::BackingUp,
        "Creating AddOns backup before addon update",
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonUpdate, TaskPhase::BackingUp)?;
    let backup_path = create_addon_backup(
        &plan.installation,
        plan.backup_output_path.clone(),
        "addon-update",
    )?;

    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::Executing,
        update_execution_message(
            plan.selected_packages.len(),
            plan.dependency_prepared_packages.len(),
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonUpdate, TaskPhase::Executing)?;

    let result = execute_update_plan(plan, backup_path, cancellation, progress)?;
    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::Completed,
        format!(
            "Addon update completed with {} written file(s)",
            result.written_files
        ),
    );
    Ok(result)
}

pub fn remove_addons(request: RemoveAddonRequest) -> AppResult<RemovedAddonPackageResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    remove_addons_task(request, &cancellation, &mut progress)
}

pub fn remove_addons_task<TCancel, TProgress>(
    request: RemoveAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<RemovedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::AddonRemove,
        TaskPhase::Preparing,
        format!(
            "Preparing addon removal from `{}`",
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonRemove, TaskPhase::Preparing)?;

    let plan = prepare_remove_addons(request)?;
    if plan.dry_run {
        let result = dry_run_remove_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonRemove,
            TaskPhase::Completed,
            format!(
                "Addon remove dry run completed for {} addon(s)",
                result.removed_addons.len()
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonRemove,
        TaskPhase::BackingUp,
        "Creating AddOns backup before addon remove",
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonRemove, TaskPhase::BackingUp)?;
    let backup_path = create_addon_backup(
        &plan.installation,
        plan.backup_output_path.clone(),
        "addon-remove",
    )?;

    emit_task_progress(
        progress,
        TaskKind::AddonRemove,
        TaskPhase::Executing,
        format!(
            "Removing {} tracked addon package(s)",
            plan.removed_packages.len()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonRemove, TaskPhase::Executing)?;

    let result = execute_remove_plan(plan, backup_path, cancellation, progress)?;
    emit_task_progress(
        progress,
        TaskKind::AddonRemove,
        TaskPhase::Completed,
        format!(
            "Addon remove completed for {} addon(s)",
            result.removed_addons.len()
        ),
    );
    Ok(result)
}

fn prepare_install_addon_with_provider<P>(
    provider: &P,
    request: InstallAddonRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<InstallAddonExecutionPlan>
where
    P: AddonProvider + ?Sized,
{
    let prepared = prepare_package_from_source_input_task_with_provider(
        provider,
        PreparePackageFromSourceInputTaskRequest {
            source: &request.source,
            context: PreparePackageTaskContext::new(
                Some(request.installation.flavor),
                request.installation.platform,
                cancellation,
                TaskKind::AddonInstall,
                TaskPhase::Preparing,
            ),
        },
        progress,
    )?;
    prepare_install_prepared_addon(InstallPreparedAddonRequest {
        installation: request.installation,
        state_paths: request.state_paths,
        prepared,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        metadata: request.metadata,
    })
}

pub(crate) fn prepare_install_prepared_addon(
    request: InstallPreparedAddonRequest,
) -> AppResult<InstallAddonExecutionPlan> {
    let registry_path = registry_path(&request.state_paths);
    let mut prepared = request.prepared;
    prepared.metadata = request.metadata;
    let files_to_write = prepared
        .addons
        .iter()
        .map(|addon| addon.file_count)
        .sum::<usize>();
    let replaced_addons = prepared
        .addons
        .iter()
        .filter_map(|addon| {
            find_existing_addon_path(
                &request.installation.addon_dir,
                &addon.addon.directory_name,
                request.installation.platform,
            )
            .transpose()
        })
        .map(|existing| existing.map(|existing| existing.name))
        .collect::<AppResult<Vec<_>>>()?;

    if !request.replace_existing && !replaced_addons.is_empty() {
        return Err(AppError::Validation(format!(
            "addon directories already exist: {}. Use `--replace-existing` or `addon update`.",
            replaced_addons.join(", ")
        )));
    }

    Ok(InstallAddonExecutionPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        prepared,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        registry_path,
        files_to_write,
        replaced_addons,
    })
}

fn dry_run_install_result(plan: InstallAddonExecutionPlan) -> InstalledAddonPackageResult {
    let InstallAddonExecutionPlan {
        prepared,
        files_to_write,
        replaced_addons,
        registry_path,
        ..
    } = plan;
    let PreparedAddonPackage {
        source,
        package_id,
        addons,
        ..
    } = prepared;

    InstalledAddonPackageResult {
        dry_run: true,
        source,
        package_id,
        addons: addons.into_iter().map(|addon| addon.addon).collect(),
        files_to_write,
        written_files: 0,
        replaced_addons,
        registry_path,
        backup_path: None,
    }
}

fn execute_install_plan<TCancel, TProgress>(
    plan: InstallAddonExecutionPlan,
    backup_path: PathBuf,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let InstallAddonExecutionPlan {
        installation,
        state_paths,
        prepared,
        replace_existing,
        registry_path,
        files_to_write,
        replaced_addons,
        ..
    } = plan;

    match install_prepared_package_task(
        &installation,
        &state_paths,
        prepared,
        replace_existing,
        TaskKind::AddonInstall,
        cancellation,
        progress,
    ) {
        Ok((package, written_files)) => Ok(InstalledAddonPackageResult {
            dry_run: false,
            source: package.source.clone(),
            package_id: package.package_id.clone(),
            addons: package.addons.clone(),
            files_to_write,
            written_files,
            replaced_addons,
            registry_path,
            backup_path: Some(backup_path),
        }),
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}

fn prepare_update_addons_with_provider<P>(
    provider: &P,
    request: UpdateAddonRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<UpdateAddonsExecutionPlan>
where
    P: AddonProvider + ?Sized,
{
    let registry_path = registry_path(&request.state_paths);
    let registry = load_registry(&request.installation, &request.state_paths)?;
    if registry.packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let policies = AddonUpdatePolicySnapshot::load(&request.installation, &request.state_paths)?;
    let mut selected_packages = select_tracked_packages(&registry, request.name.as_deref())?;
    let ignored_packages = if request.name.is_some() {
        Vec::new()
    } else {
        let mut ignored = Vec::new();
        selected_packages.retain(|package| {
            if policies.is_ignored(package) {
                ignored.push(package.package_id.clone());
                false
            } else {
                true
            }
        });
        ignored.sort();
        ignored
    };
    let mut prepared_packages = Vec::new();
    let mut dependency_prepared_packages = Vec::new();
    let mut planned_dependency_keys = BTreeSet::new();
    for package in &selected_packages {
        let package_policy = policies.provider_update_policy(package)?;
        let mut prepared = prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &package_policy.effective_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    TaskKind::AddonUpdate,
                    TaskPhase::Preparing,
                ),
            )
            .with_resolution_policy(package_policy.resolution_policy),
            progress,
        )?;
        prepared.package_id = package.package_id.clone();
        prepared_packages.push(prepared);

        if package_policy.install_dependencies {
            collect_missing_dependency_prepared_packages(
                provider,
                MissingDependencyCollectionRequest {
                    source: &package_policy.effective_source,
                    resolution_policy: package_policy.resolution_policy,
                    installation: &request.installation,
                    registry: &registry,
                    selected_packages: &selected_packages,
                    task_kind: TaskKind::AddonUpdate,
                },
                &mut MissingDependencyCollectionState {
                    prepared_packages: &mut dependency_prepared_packages,
                    planned_keys: &mut planned_dependency_keys,
                },
                cancellation,
                progress,
            )?;
        }
    }

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

    Ok(UpdateAddonsExecutionPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        registry,
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
        ignored_packages,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        registry_path,
        files_to_write,
    })
}

fn no_op_update_result(plan: UpdateAddonsExecutionPlan) -> UpdatedAddonPackageResult {
    UpdatedAddonPackageResult {
        dry_run: plan.dry_run,
        registry_path: plan.registry_path,
        files_to_write: 0,
        written_files: 0,
        updated_packages: Vec::new(),
        installed_dependency_packages: Vec::new(),
        ignored_packages: plan.ignored_packages,
        backup_path: None,
    }
}

fn dry_run_update_result(plan: UpdateAddonsExecutionPlan) -> UpdatedAddonPackageResult {
    let UpdateAddonsExecutionPlan {
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
        ignored_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    UpdatedAddonPackageResult {
        dry_run: true,
        registry_path,
        files_to_write,
        written_files: 0,
        updated_packages: prepared_packages
            .into_iter()
            .zip(selected_packages.iter())
            .map(|(package, selected)| {
                let metadata = package
                    .metadata
                    .clone()
                    .or_else(|| selected.metadata.clone());
                TrackedAddonPackage {
                    package_id: package.package_id,
                    source: package.source,
                    installed_at: selected.installed_at.clone(),
                    updated_at: String::new(),
                    addons: package
                        .addons
                        .into_iter()
                        .map(|addon| addon.addon)
                        .collect(),
                    metadata,
                }
            })
            .collect(),
        installed_dependency_packages: preview_installed_dependency_packages(
            &dependency_prepared_packages,
        ),
        ignored_packages,
        backup_path: None,
    }
}

fn execute_update_plan<TCancel, TProgress>(
    plan: UpdateAddonsExecutionPlan,
    backup_path: PathBuf,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UpdatedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let UpdateAddonsExecutionPlan {
        installation,
        state_paths,
        registry,
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
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
            selected_packages,
            prepared_packages,
            dependency_prepared_packages,
            task: TaskKind::AddonUpdate,
        },
        cancellation,
        progress,
    ) {
        Ok((updated_packages, installed_dependency_packages, written_files)) => {
            Ok(UpdatedAddonPackageResult {
                dry_run: false,
                registry_path,
                files_to_write,
                written_files,
                updated_packages,
                installed_dependency_packages,
                ignored_packages,
                backup_path: Some(backup_path),
            })
        }
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}

fn update_execution_message(updated_count: usize, dependency_count: usize) -> String {
    match dependency_count {
        0 => format!("Updating {updated_count} tracked addon package(s)"),
        _ => format!(
            "Updating {updated_count} tracked addon package(s) and installing {dependency_count} dependency package(s)"
        ),
    }
}

fn prepare_remove_addons(request: RemoveAddonRequest) -> AppResult<RemoveAddonsExecutionPlan> {
    let registry_path = registry_path(&request.state_paths);
    let registry = load_registry(&request.installation, &request.state_paths)?;
    if registry.packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let removed_packages = select_tracked_packages(&registry, Some(&request.name))?;
    let removed_addons = removed_packages
        .iter()
        .flat_map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.directory_name.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    Ok(RemoveAddonsExecutionPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        removed_packages,
        removed_addons,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        registry_path,
    })
}

fn dry_run_remove_result(plan: RemoveAddonsExecutionPlan) -> RemovedAddonPackageResult {
    RemovedAddonPackageResult {
        dry_run: true,
        registry_path: plan.registry_path,
        removed_packages: plan.removed_packages,
        removed_addons: plan.removed_addons,
        registry_cleaned: false,
        backup_path: None,
    }
}

fn execute_remove_plan<TCancel, TProgress>(
    plan: RemoveAddonsExecutionPlan,
    backup_path: PathBuf,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<RemovedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let RemoveAddonsExecutionPlan {
        installation,
        state_paths,
        removed_packages,
        removed_addons,
        registry_path,
        ..
    } = plan;

    match remove_selected_packages_task(
        &installation,
        &state_paths,
        removed_packages.clone(),
        TaskKind::AddonRemove,
        cancellation,
        progress,
    ) {
        Ok(registry_cleaned) => Ok(RemovedAddonPackageResult {
            dry_run: false,
            registry_path,
            removed_packages,
            removed_addons,
            registry_cleaned,
            backup_path: Some(backup_path),
        }),
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}

fn create_addon_backup(
    installation: &DetectedFlavorInstallation,
    output_path: Option<PathBuf>,
    label: &str,
) -> AppResult<PathBuf> {
    Ok(create_backup(BackupRequest {
        installation: installation.clone(),
        output_path,
        groups: vec![BackupGroup::Addons],
        label: Some(label.to_string()),
    })?
    .archive_path)
}
