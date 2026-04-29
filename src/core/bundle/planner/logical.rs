use std::path::Path;

use super::super::apply_model::planned::PlannedEntry;
use super::super::apply_policy::cleanup::{build_cleanup_operations, cleanup_scope_for_entry};
use super::super::apply_policy::policy::resource_policy_for_group;
use super::super::character_mapping::build_character_mappings;
use super::super::entry_plan::context::plan_extractable_entries;
use super::super::target_accounts::compatibility::validate_target_compatibility;
use super::super::target_accounts::selection::resolve_selected_target_accounts;
use super::super::types::apply::BundleApplyMappings;
use super::model::{LogicalBundleApply, LogicalEntryDisposition, LogicalEntryOperation};
use crate::core::archive_path::{
    PlatformPathCollisionKind, PlatformPathPrefixConflictKind, find_platform_path_collision,
    find_platform_path_prefix_conflict,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, LocalWowAccount, discover_local_accounts};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{BundleManifest, ResourceApplyPolicy};

pub(super) fn plan_apply_from_entries(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    entry_names: &[String],
    apply_mappings: &BundleApplyMappings,
) -> AppResult<LogicalBundleApply> {
    manifest.validate()?;
    apply_mappings.validate()?;
    validate_target_compatibility(&manifest, installation)?;
    let discovered_accounts = discover_local_accounts(installation)?;
    let character_mappings = build_character_mappings(&manifest, apply_mappings)?;
    let selected_target_accounts = resolve_selected_target_accounts(
        &manifest,
        &discovered_accounts,
        &character_mappings,
        apply_mappings,
    )?;
    let planned_entries = plan_extractable_entries(
        entry_names,
        installation,
        &manifest,
        &character_mappings,
        apply_mappings,
        &selected_target_accounts,
    )?;
    validate_planned_destination_collisions(&planned_entries, installation)?;

    build_logical_apply(
        plan_path,
        installation,
        manifest,
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        planned_entries,
    )
}

fn validate_planned_destination_collisions(
    planned_entries: &[PlannedEntry],
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    if let Some(collision) =
        find_platform_path_collision(planned_entries.iter(), installation.platform, |entry| {
            entry.destination.as_path()
        })
    {
        return match collision.kind {
            PlatformPathCollisionKind::Exact => Err(AppError::Validation(format!(
                "bundle archive maps multiple entries onto the same target path: `{}` and `{}` -> {}",
                collision.previous.archive_name,
                collision.current.archive_name,
                collision.current.destination.display()
            ))),
            PlatformPathCollisionKind::CaseInsensitive => Err(AppError::Validation(format!(
                "bundle archive contains case-insensitive target path collisions: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
                collision.previous.archive_name,
                collision.previous.destination.display(),
                collision.current.archive_name,
                collision.current.destination.display()
            ))),
        };
    }

    let Some(conflict) = find_platform_path_prefix_conflict(
        planned_entries.iter(),
        installation.platform,
        |entry| entry.destination.as_path(),
    ) else {
        return Ok(());
    };

    match conflict.kind {
        PlatformPathPrefixConflictKind::Exact => Err(AppError::Validation(format!(
            "bundle archive contains conflicting file and directory target paths: `{}` -> {} and `{}` -> {}",
            conflict.ancestor.archive_name,
            conflict.ancestor.destination.display(),
            conflict.descendant.archive_name,
            conflict.descendant.destination.display()
        ))),
        PlatformPathPrefixConflictKind::CaseInsensitive => Err(AppError::Validation(format!(
            "bundle archive contains case-insensitive file and directory target path conflicts: `{}` -> {} and `{}` -> {} would create file/directory collisions on Windows/default macOS targets",
            conflict.ancestor.archive_name,
            conflict.ancestor.destination.display(),
            conflict.descendant.archive_name,
            conflict.descendant.destination.display()
        ))),
    }
}

fn build_logical_apply(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    discovered_accounts: Vec<LocalWowAccount>,
    selected_target_accounts: Vec<String>,
    character_mappings: Vec<CharacterMapping>,
    planned_entries: Vec<PlannedEntry>,
) -> AppResult<LogicalBundleApply> {
    let cleanup_operations = build_cleanup_operations(&planned_entries, &manifest, installation)?;
    let cleanup_roots = cleanup_operations
        .iter()
        .map(|operation| operation.destination.clone())
        .collect::<Vec<_>>();
    let mut entry_operations = Vec::with_capacity(planned_entries.len());

    for entry in planned_entries {
        let policy = resource_policy_for_group(&manifest, entry.group);
        let disposition = if policy == ResourceApplyPolicy::Preserve
            || (policy == ResourceApplyPolicy::Share && entry.destination.exists())
        {
            LogicalEntryDisposition::Preserve
        } else {
            let cleanup_root = cleanup_scope_for_entry(&entry, installation)?;
            let will_cleanup = cleanup_root
                .as_ref()
                .is_some_and(|root| cleanup_roots.iter().any(|candidate| candidate == root));
            LogicalEntryDisposition::Materialize { will_cleanup }
        };

        entry_operations.push(LogicalEntryOperation { entry, disposition });
    }

    Ok(LogicalBundleApply {
        plan_path: plan_path.to_path_buf(),
        target_flavor_root: installation.flavor_root.clone(),
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        manifest,
        cleanup_operations,
        entry_operations,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::validate_planned_destination_collisions;
    use crate::core::bundle::apply_model::planned::PlannedEntry;
    use crate::core::bundle::types::apply::ApplyGroup;
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

    #[test]
    fn validate_planned_destination_collisions_rejects_case_insensitive_prefix_conflicts() {
        let installation = fixture_installation(HostPlatform::MacOs);
        let error = validate_planned_destination_collisions(
            &[
                planned_entry("addons/WeakAuras", "Interface/AddOns/WeakAuras"),
                planned_entry(
                    "addons/weakauras/Config.lua",
                    "Interface/AddOns/weakauras/Config.lua",
                ),
            ],
            &installation,
        )
        .expect_err("case-insensitive file/directory conflicts should fail");

        assert!(
            error
                .to_string()
                .contains("case-insensitive file and directory target path conflicts")
        );
    }

    #[test]
    fn validate_planned_destination_collisions_allows_case_distinct_prefixes_on_linux() {
        let installation = fixture_installation(HostPlatform::Linux);
        validate_planned_destination_collisions(
            &[
                planned_entry("addons/WeakAuras", "Interface/AddOns/WeakAuras"),
                planned_entry(
                    "addons/weakauras/Config.lua",
                    "Interface/AddOns/weakauras/Config.lua",
                ),
            ],
            &installation,
        )
        .expect("linux should allow case-distinct file/directory paths");
    }

    fn planned_entry(archive_name: &str, destination: &str) -> PlannedEntry {
        PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: PathBuf::from(destination),
            rewrites: Vec::new(),
            group: ApplyGroup::Addons,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }
    }

    fn fixture_installation(platform: HostPlatform) -> DetectedFlavorInstallation {
        let root = PathBuf::from("fixture");
        let product_root = root.join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");

        DetectedFlavorInstallation {
            platform,
            product_root,
            flavor_root: flavor_root.clone(),
            flavor: WowFlavor::Retail,
            interface_dir: interface_dir.clone(),
            addon_dir: interface_dir.join("AddOns"),
            wtf_dir: flavor_root.join("WTF"),
            fonts_dir: flavor_root.join("Fonts"),
        }
    }
}
