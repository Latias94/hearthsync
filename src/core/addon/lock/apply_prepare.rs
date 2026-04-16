use super::apply_model::{
    MetadataOnlyAddonLockAction, PreparedAddonLockApply, metadata_from_lock_package,
};
use super::plan::AddonLockPlanContext;
use super::source_resolution::prepare_expected_lock_package;
use super::*;

pub(super) fn prepare_addon_lock_apply(
    plan: &AddonLockPlanContext,
    source_overrides: &BTreeMap<String, PathBuf>,
    installation: &DetectedFlavorInstallation,
) -> AppResult<PreparedAddonLockApply> {
    let mut remove_packages = Vec::new();
    let mut update_current_packages = Vec::new();
    let mut update_prepared_packages = Vec::new();
    let mut install_prepared_packages = Vec::new();
    let mut metadata_actions = Vec::new();

    for action in &plan.actions {
        match action.action.kind {
            AddonLockSyncActionKind::Remove => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock remove action is missing current package".to_string(),
                    )
                })?;
                remove_packages.push(current.clone());
            }
            AddonLockSyncActionKind::Update => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock update action is missing current package".to_string(),
                    )
                })?;
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock update action is missing expected package".to_string(),
                    )
                })?;
                let mut prepared = prepare_expected_lock_package(
                    expected,
                    source_overrides
                        .get(&action.action.comparison_key)
                        .map(PathBuf::as_path),
                    installation.flavor,
                )?;
                prepared.metadata = metadata_from_lock_package(expected);
                update_current_packages.push(current.clone());
                update_prepared_packages.push(prepared);
            }
            AddonLockSyncActionKind::Install => {
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock install action is missing expected package".to_string(),
                    )
                })?;
                let mut prepared = prepare_expected_lock_package(
                    expected,
                    source_overrides
                        .get(&action.action.comparison_key)
                        .map(PathBuf::as_path),
                    installation.flavor,
                )?;
                prepared.metadata = metadata_from_lock_package(expected);
                install_prepared_packages.push(prepared);
            }
            AddonLockSyncActionKind::MetadataOnly => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock metadata-only action is missing current package".to_string(),
                    )
                })?;
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock metadata-only action is missing expected package".to_string(),
                    )
                })?;
                metadata_actions.push(MetadataOnlyAddonLockAction {
                    current: current.clone(),
                    expected: expected.clone(),
                });
            }
        }
    }

    Ok(PreparedAddonLockApply {
        remove_packages,
        update_current_packages,
        update_prepared_packages,
        install_prepared_packages,
        metadata_actions,
    })
}
