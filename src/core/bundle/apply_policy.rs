use std::collections::BTreeMap;

use super::*;

pub(super) fn build_cleanup_operations(
    planned_entries: &[PlannedEntry],
    manifest: &BundleManifest,
    installation: &DetectedFlavorInstallation,
) -> AppResult<Vec<PlannedCleanup>> {
    let mut cleanup_roots = BTreeMap::<PathBuf, PlannedCleanup>::new();

    for entry in planned_entries {
        let policy = resource_policy_for_group(manifest, entry.group);
        if !policy_requires_cleanup(policy) {
            continue;
        }

        let Some(destination) = cleanup_scope_for_entry(entry, installation)? else {
            continue;
        };
        if !destination.exists() {
            continue;
        }

        cleanup_roots
            .entry(destination.clone())
            .or_insert_with(|| PlannedCleanup {
                group: entry.group,
                destination,
                target_account: entry.target_account.clone(),
                target_server: entry.target_server.clone(),
                target_character: entry.target_character.clone(),
            });
    }

    Ok(cleanup_roots.into_values().collect())
}

pub(super) fn cleanup_scope_for_entry(
    entry: &PlannedEntry,
    installation: &DetectedFlavorInstallation,
) -> AppResult<Option<PathBuf>> {
    match entry.group {
        ApplyGroup::Addons => {
            let relative = entry
                .destination
                .strip_prefix(&installation.addon_dir)
                .map_err(|error| AppError::Validation(error.to_string()))?;
            let mut components = relative.components();
            let Some(component) = components.next() else {
                return Ok(None);
            };
            Ok(Some(installation.addon_dir.join(component.as_os_str())))
        }
        ApplyGroup::WtfCommon => match entry.wtf_scope.unwrap_or(WtfScope::Unknown) {
            WtfScope::GlobalConfig => Ok(Some(installation.wtf_dir.join("Config.wtf"))),
            WtfScope::AccountSavedVariables => {
                let target_account = entry.target_account.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "wtf common cleanup root requires a target account".to_string(),
                    )
                })?;
                Ok(Some(
                    installation
                        .wtf_dir
                        .join("Account")
                        .join(target_account)
                        .join("SavedVariables"),
                ))
            }
            WtfScope::AccountRootFile | WtfScope::CacheLike | WtfScope::Unknown => {
                Ok(Some(entry.destination.clone()))
            }
            WtfScope::CharacterSavedVariables | WtfScope::CharacterState => {
                Err(AppError::Validation(
                    "character WTF scope cannot be used for common WTF cleanup".to_string(),
                ))
            }
        },
        ApplyGroup::WtfCharacters => {
            let target_account = entry.target_account.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "wtf character cleanup root requires a target account".to_string(),
                )
            })?;
            let target_server = entry.target_server.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "wtf character cleanup root requires a target server".to_string(),
                )
            })?;
            let target_character = entry.target_character.as_ref().ok_or_else(|| {
                AppError::Validation(
                    "wtf character cleanup root requires a target character".to_string(),
                )
            })?;
            Ok(Some(
                installation
                    .wtf_dir
                    .join("Account")
                    .join(target_account)
                    .join(target_server)
                    .join(target_character),
            ))
        }
        ApplyGroup::Fonts => Ok(Some(installation.fonts_dir.clone())),
        ApplyGroup::InterfaceAssets => {
            let relative = entry
                .destination
                .strip_prefix(&installation.interface_dir)
                .map_err(|error| AppError::Validation(error.to_string()))?;
            let mut components = relative.components();
            let Some(component) = components.next() else {
                return Ok(None);
            };
            Ok(Some(installation.interface_dir.join(component.as_os_str())))
        }
        ApplyGroup::Metadata => Ok(None),
    }
}

pub(super) fn resource_policy_for_group(
    manifest: &BundleManifest,
    group: ApplyGroup,
) -> ResourceApplyPolicy {
    match group {
        ApplyGroup::Addons => manifest.apply.addons,
        ApplyGroup::WtfCommon => manifest.apply.wtf_common,
        ApplyGroup::WtfCharacters => manifest.apply.wtf_characters,
        ApplyGroup::Fonts => manifest.apply.fonts,
        ApplyGroup::InterfaceAssets => manifest.apply.interface_assets,
        ApplyGroup::Metadata => ResourceApplyPolicy::Merge,
    }
}

pub(super) fn policy_requires_cleanup(policy: ResourceApplyPolicy) -> bool {
    matches!(
        policy,
        ResourceApplyPolicy::Sync
            | ResourceApplyPolicy::Mirror
            | ResourceApplyPolicy::ReplaceSelected
    )
}

pub(super) fn apply_action_order(action: ApplyAction) -> u8 {
    match action {
        ApplyAction::Remove => 0,
        ApplyAction::Add => 1,
        ApplyAction::Replace => 2,
        ApplyAction::Skip => 3,
        ApplyAction::Preserve => 4,
    }
}

pub(super) fn apply_group_order(group: ApplyGroup) -> u8 {
    match group {
        ApplyGroup::Addons => 0,
        ApplyGroup::InterfaceAssets => 1,
        ApplyGroup::Fonts => 2,
        ApplyGroup::WtfCommon => 3,
        ApplyGroup::WtfCharacters => 4,
        ApplyGroup::Metadata => 5,
    }
}
