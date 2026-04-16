use super::apply_model::{
    MetadataOnlyAddonLockAction, PreparedAddonLockApply, metadata_from_lock_package,
};
use super::storage::now_rfc3339;
use super::*;

pub(super) fn execute_prepared_addon_lock_apply(
    installation: &DetectedFlavorInstallation,
    prepared: PreparedAddonLockApply,
    replace_existing: bool,
) -> AppResult<()> {
    if !prepared.remove_packages.is_empty() {
        remove_selected_packages(installation, prepared.remove_packages)?;
    }

    if !prepared.update_current_packages.is_empty() {
        let registry = load_registry(installation)?;
        update_prepared_packages(
            installation,
            registry,
            prepared.update_current_packages,
            prepared.update_prepared_packages,
        )?;
    }

    for prepared_package in prepared.install_prepared_packages {
        install_prepared_package(installation, prepared_package, replace_existing)?;
    }

    if !prepared.metadata_actions.is_empty() {
        apply_metadata_only_actions(installation, prepared.metadata_actions)?;
    }

    Ok(())
}

fn apply_metadata_only_actions(
    installation: &DetectedFlavorInstallation,
    actions: Vec<MetadataOnlyAddonLockAction>,
) -> AppResult<()> {
    let mut registry = load_registry(installation)?;
    let timestamp = now_rfc3339()?;

    for action in actions {
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

    save_registry(installation, &registry)
}
