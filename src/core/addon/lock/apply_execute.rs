use super::apply_model::{
    MetadataOnlyAddonLockAction, PreparedAddonLockApply, metadata_from_lock_package,
};
use super::storage::now_rfc3339;
use crate::core::addon::{
    AddonStatePaths, install_prepared_package_task, load_registry, remove_selected_packages_task,
    save_registry, update_prepared_packages_task,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, TaskKind, TaskPhase, TaskProgressCode, TaskProgressSink,
    emit_task_step_progress, ensure_task_not_cancelled,
};

pub(super) fn execute_prepared_addon_lock_apply<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    prepared: PreparedAddonLockApply,
    replace_existing: bool,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<()>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    if !prepared.remove_packages.is_empty() {
        remove_selected_packages_task(
            installation,
            state_paths,
            prepared.remove_packages,
            TaskKind::AddonLockApply,
            cancellation,
            progress,
        )?;
    }

    if !prepared.update_current_packages.is_empty() {
        let registry = load_registry(installation, state_paths)?;
        update_prepared_packages_task(
            installation,
            state_paths,
            registry,
            prepared.update_current_packages,
            prepared.update_prepared_packages,
            TaskKind::AddonLockApply,
            cancellation,
            progress,
        )?;
    }

    for prepared_package in prepared.install_prepared_packages {
        install_prepared_package_task(
            installation,
            state_paths,
            prepared_package,
            replace_existing,
            TaskKind::AddonLockApply,
            cancellation,
            progress,
        )?;
    }

    if !prepared.metadata_actions.is_empty() {
        apply_metadata_only_actions(
            installation,
            state_paths,
            prepared.metadata_actions,
            cancellation,
            progress,
        )?;
    }

    Ok(())
}

fn apply_metadata_only_actions<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    actions: Vec<MetadataOnlyAddonLockAction>,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<()>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let mut registry = load_registry(installation, state_paths)?;
    let timestamp = now_rfc3339()?;
    let total_actions = actions.len();

    for (index, action) in actions.into_iter().enumerate() {
        ensure_task_not_cancelled(cancellation, TaskKind::AddonLockApply, TaskPhase::Executing)?;
        emit_task_step_progress(
            progress,
            TaskKind::AddonLockApply,
            TaskPhase::Executing,
            TaskProgressCode::ApplyMetadata,
            index + 1,
            total_actions,
            format!(
                "Applying metadata-only lock action {}/{} `{}`",
                index + 1,
                total_actions,
                action.current.package_id
            ),
        );
        let package = registry
            .packages
            .iter_mut()
            .find(|candidate| **candidate == action.current)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked package disappeared before metadata apply: {}",
                    action.current.package_id
                ))
            })?;
        package.package_id = action.expected.package_id.clone();
        package.updated_at = timestamp.clone();
        package.metadata = metadata_from_lock_package(&action.expected);
    }

    save_registry(installation, state_paths, &registry)
}
