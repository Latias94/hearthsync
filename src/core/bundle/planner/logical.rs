use std::collections::BTreeMap;
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
use crate::core::archive_path::platform_path_collision_key;
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
    let mut seen = BTreeMap::<String, &PlannedEntry>::new();

    for entry in planned_entries {
        let key = platform_path_collision_key(&entry.destination, installation.platform);
        let Some(previous) = seen.insert(key, entry) else {
            continue;
        };

        if previous.destination == entry.destination {
            return Err(AppError::Validation(format!(
                "bundle archive maps multiple entries onto the same target path: `{}` and `{}` -> {}",
                previous.archive_name,
                entry.archive_name,
                entry.destination.display()
            )));
        }

        return Err(AppError::Validation(format!(
            "bundle archive contains case-insensitive target path collisions: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
            previous.archive_name,
            previous.destination.display(),
            entry.archive_name,
            entry.destination.display()
        )));
    }

    Ok(())
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
