use std::collections::BTreeMap;
use std::path::PathBuf;

use super::super::*;
use super::policy::resource_policy_for_group;

pub(in crate::core::bundle) fn build_cleanup_operations(
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

pub(in crate::core::bundle) fn cleanup_scope_for_entry(
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
            WtfScope::RootSavedVariables => Ok(Some(
                installation.wtf_dir.join("Account").join("SavedVariables"),
            )),
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

fn policy_requires_cleanup(policy: ResourceApplyPolicy) -> bool {
    matches!(
        policy,
        ResourceApplyPolicy::Sync
            | ResourceApplyPolicy::Mirror
            | ResourceApplyPolicy::ReplaceSelected
    )
}
