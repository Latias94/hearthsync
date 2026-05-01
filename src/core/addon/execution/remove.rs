use std::collections::BTreeSet;
use std::path::PathBuf;

use super::super::registry::registry_path;
use super::super::{
    AddonStatePaths, RemoveAddonRequest, RemovedAddonPackageResult, TrackedAddonPackage,
    load_registry, no_tracked_packages_error, remove_selected_packages_task,
    rollback_or_report_addon_error, select_tracked_packages,
};
use super::backup::create_addon_backup;
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};
struct RemoveAddonsExecutionPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    removed_packages: Vec<TrackedAddonPackage>,
    removed_addons: Vec<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
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
