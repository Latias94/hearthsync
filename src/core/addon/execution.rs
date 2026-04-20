use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};

use super::registry::registry_path;
use super::{
    AddonPackageMetadata, AddonProvider, AddonRegistry, DefaultAddonProvider, InstallAddonRequest,
    InstalledAddonPackageResult, PreparedAddonPackage, RemoveAddonRequest,
    RemovedAddonPackageResult, TrackedAddonPackage, UpdateAddonRequest, UpdatedAddonPackageResult,
    install_prepared_package_task, load_registry, prepare_package_from_source_input_with_provider,
    prepare_package_from_source_ref_with_provider, remove_selected_packages_task,
    rollback_or_report_addon_error, update_prepared_packages_task,
};

#[derive(Debug)]
pub(crate) struct InstallPreparedAddonRequest {
    pub(crate) installation: DetectedFlavorInstallation,
    pub(crate) prepared: PreparedAddonPackage,
    pub(crate) dry_run: bool,
    pub(crate) backup_output_path: Option<PathBuf>,
    pub(crate) replace_existing: bool,
    pub(crate) metadata: Option<AddonPackageMetadata>,
}

pub(crate) struct InstallAddonExecutionPlan {
    installation: DetectedFlavorInstallation,
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
    registry: AddonRegistry,
    selected_packages: Vec<TrackedAddonPackage>,
    prepared_packages: Vec<PreparedAddonPackage>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
    files_to_write: usize,
}

struct RemoveAddonsExecutionPlan {
    installation: DetectedFlavorInstallation,
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

    let plan = prepare_install_addon_with_provider(provider, request, cancellation)?;
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

    let plan = prepare_update_addons_with_provider(provider, request, cancellation)?;
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
        format!(
            "Updating {} tracked addon package(s)",
            plan.selected_packages.len()
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
) -> AppResult<InstallAddonExecutionPlan>
where
    P: AddonProvider + ?Sized,
{
    let prepared = prepare_package_from_source_input_with_provider(
        provider,
        &request.source,
        Some(request.installation.flavor),
        cancellation,
    )?;
    prepare_install_prepared_addon(InstallPreparedAddonRequest {
        installation: request.installation,
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
    let registry_path = registry_path(&request.installation);
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
        .filter(|addon| {
            request
                .installation
                .addon_dir
                .join(&addon.addon.directory_name)
                .exists()
        })
        .map(|addon| addon.addon.directory_name.clone())
        .collect::<Vec<_>>();

    if !request.replace_existing && !replaced_addons.is_empty() {
        return Err(AppError::Validation(format!(
            "addon directories already exist: {}. Use `--replace-existing` or `addon update`.",
            replaced_addons.join(", ")
        )));
    }

    Ok(InstallAddonExecutionPlan {
        installation: request.installation,
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
        prepared,
        replace_existing,
        registry_path,
        files_to_write,
        replaced_addons,
        ..
    } = plan;

    match install_prepared_package_task(
        &installation,
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
) -> AppResult<UpdateAddonsExecutionPlan>
where
    P: AddonProvider + ?Sized,
{
    let registry_path = registry_path(&request.installation);
    let registry = load_registry(&request.installation)?;
    if registry.packages.is_empty() {
        return Err(AppError::Validation(
            "no tracked addon packages found. Use `addon install` first.".to_string(),
        ));
    }

    let selected_packages = select_packages_for_update(&registry, request.name.as_deref())?;
    let mut prepared_packages = Vec::new();
    for package in &selected_packages {
        prepared_packages.push(prepare_package_from_source_ref_with_provider(
            provider,
            &package.source,
            Some(request.installation.flavor),
            cancellation,
        )?);
    }

    let files_to_write = prepared_packages
        .iter()
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
        registry,
        selected_packages,
        prepared_packages,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        registry_path,
        files_to_write,
    })
}

fn dry_run_update_result(plan: UpdateAddonsExecutionPlan) -> UpdatedAddonPackageResult {
    let UpdateAddonsExecutionPlan {
        selected_packages,
        prepared_packages,
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
        registry,
        selected_packages,
        prepared_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    match update_prepared_packages_task(
        &installation,
        registry,
        selected_packages,
        prepared_packages,
        TaskKind::AddonUpdate,
        cancellation,
        progress,
    ) {
        Ok((updated_packages, written_files)) => Ok(UpdatedAddonPackageResult {
            dry_run: false,
            registry_path,
            files_to_write,
            written_files,
            updated_packages,
            backup_path: Some(backup_path),
        }),
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}

fn prepare_remove_addons(request: RemoveAddonRequest) -> AppResult<RemoveAddonsExecutionPlan> {
    let registry_path = registry_path(&request.installation);
    let registry = load_registry(&request.installation)?;
    if registry.packages.is_empty() {
        return Err(AppError::Validation(
            "no tracked addon packages found. Use `addon install` first.".to_string(),
        ));
    }

    let removed_packages = select_packages_for_update(&registry, Some(&request.name))?;
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
        removed_packages,
        removed_addons,
        registry_path,
        ..
    } = plan;

    match remove_selected_packages_task(
        &installation,
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

fn select_packages_for_update(
    registry: &AddonRegistry,
    name: Option<&str>,
) -> AppResult<Vec<TrackedAddonPackage>> {
    match name {
        None => Ok(registry.packages.clone()),
        Some(name) => {
            let mut matches = registry
                .packages
                .iter()
                .filter(|package| {
                    package.package_id.eq_ignore_ascii_case(name)
                        || package
                            .addons
                            .iter()
                            .any(|addon| addon.directory_name.eq_ignore_ascii_case(name))
                })
                .cloned()
                .collect::<Vec<_>>();
            matches.sort_by(|left, right| left.package_id.cmp(&right.package_id));
            if matches.is_empty() {
                return Err(AppError::NotFound(format!(
                    "no tracked addon package matched `{name}`"
                )));
            }
            Ok(matches)
        }
    }
}
