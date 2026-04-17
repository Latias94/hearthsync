use std::collections::BTreeSet;
use std::path::PathBuf;

use super::matching::match_index_package_to_tracked_package;
use super::storage::{ensure_package_supports_flavor, find_index_package, load_addon_index};
use super::*;
use crate::core::addon::{
    AddonPackageMetadata, AddonRegistry, InstallAddonRequest, PreparedAddonPackage,
    TrackedAddonPackage, UpdatedAddonPackageResult, install_addon_task, list_addons, load_registry,
    prepare_package_from_source_ref_with_flavor, rollback_or_report_addon_error,
    update_prepared_packages,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressEvent,
    TaskProgressSink, emit_task_progress, ensure_task_not_cancelled,
};

struct IndexInstallPlan {
    index_path: PathBuf,
    package: AddonIndexPackage,
    install_request: InstallAddonRequest,
}

struct IndexUpdatePlan {
    installation: DetectedFlavorInstallation,
    index_path: PathBuf,
    selected_packages: Vec<AddonIndexPackage>,
    registry: AddonRegistry,
    prepared_packages: Vec<PreparedAddonPackage>,
    matched_packages: Vec<TrackedAddonPackage>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
    files_to_write: usize,
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
    let plan = prepare_index_install(request)?;
    let mut remapped_progress = RemappedTaskProgressSink {
        inner: progress,
        task: TaskKind::AddonIndexInstall,
    };
    let install = install_addon_task(plan.install_request, cancellation, &mut remapped_progress)
        .map_err(|error| {
            remap_cancelled_task_kind(error, TaskKind::AddonInstall, TaskKind::AddonIndexInstall)
        })?;

    Ok(AddonIndexInstallResult {
        index_path: plan.index_path,
        package: plan.package,
        install,
    })
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

    let plan = prepare_index_update(request)?;
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
        format!(
            "Updating {} addon index package(s)",
            plan.selected_packages.len()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Executing,
    )?;
    let result = execute_index_update_plan(plan, backup_path)?;

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

fn metadata_from_index_package(
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

fn prepare_index_install(request: AddonIndexInstallRequest) -> AppResult<IndexInstallPlan> {
    let index = load_addon_index(&request.index_path)?;
    let package = find_index_package(&index, &request.name)?.clone();
    ensure_package_supports_flavor(&package, request.installation.flavor.as_str())?;

    Ok(IndexInstallPlan {
        index_path: request.index_path,
        package: package.clone(),
        install_request: InstallAddonRequest {
            installation: request.installation,
            source: package.source.display_name(),
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            metadata: Some(metadata_from_index_package(&index, &package)),
        },
    })
}

fn prepare_index_update(request: AddonIndexUpdateRequest) -> AppResult<IndexUpdatePlan> {
    let index = load_addon_index(&request.index_path)?;
    let selected_packages = match &request.name {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    for package in &selected_packages {
        ensure_package_supports_flavor(package, request.installation.flavor.as_str())?;
    }

    let inventory = list_addons(&request.installation)?;
    if inventory.tracked_packages.is_empty() {
        return Err(AppError::Validation(
            "no tracked addon packages found. Use `addon index install` or `addon install` first."
                .to_string(),
        ));
    }

    let registry = load_registry(&request.installation)?;
    let mut prepared_packages = Vec::new();
    let mut matched_packages = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    for package in &selected_packages {
        let mut prepared = prepare_package_from_source_ref_with_flavor(
            &package.source,
            Some(request.installation.flavor),
        )?;
        prepared.metadata = Some(metadata_from_index_package(&index, package));
        let matched = match_index_package_to_tracked_package(
            package,
            &prepared,
            &inventory.tracked_packages,
            &used_package_ids,
        )?;
        used_package_ids.insert(matched.package_id.clone());
        prepared_packages.push(prepared);
        matched_packages.push(matched);
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

    Ok(IndexUpdatePlan {
        installation: request.installation,
        index_path: request.index_path,
        selected_packages,
        registry,
        prepared_packages,
        matched_packages,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        registry_path: inventory.registry_path,
        files_to_write,
    })
}

fn dry_run_index_update_result(plan: IndexUpdatePlan) -> AddonIndexUpdateResult {
    let IndexUpdatePlan {
        index_path,
        selected_packages,
        prepared_packages,
        matched_packages,
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
            backup_path: None,
        },
    }
}

fn execute_index_update_plan(
    plan: IndexUpdatePlan,
    backup_path: PathBuf,
) -> AppResult<AddonIndexUpdateResult> {
    let IndexUpdatePlan {
        installation,
        index_path,
        selected_packages,
        registry,
        prepared_packages,
        matched_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    match update_prepared_packages(&installation, registry, matched_packages, prepared_packages) {
        Ok((updated_packages, written_files)) => Ok(AddonIndexUpdateResult {
            index_path,
            selected_packages,
            update: UpdatedAddonPackageResult {
                dry_run: false,
                registry_path,
                files_to_write,
                written_files,
                updated_packages,
                backup_path: Some(backup_path),
            },
        }),
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
}
