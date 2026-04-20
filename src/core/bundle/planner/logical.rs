use std::path::Path;

use super::super::*;
use super::model::{LogicalBundleApply, LogicalEntryDisposition, LogicalEntryOperation};

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
