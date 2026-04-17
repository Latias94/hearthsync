use std::fs::File;
use std::path::Path;

use zip::ZipArchive;

use super::*;

pub fn plan_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<BundleApplyPlan> {
    Ok(prepare_bundle_apply(bundle_path, installation, apply_mappings)?.plan)
}

pub(super) fn prepare_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<PreparedBundleApply> {
    let inspection = inspect_bundle(bundle_path)?;
    let entry_names = collect_bundle_entry_names(bundle_path)?;
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;

    prepare_apply_from_entries(
        bundle_path,
        installation,
        inspection.manifest,
        &entry_names,
        apply_mappings,
        PreparedApplySource::BundleArchive {
            bundle_path: bundle_path.to_path_buf(),
        },
        |archive_name| read_bundle_entry_bytes_from_archive(&mut archive, archive_name),
        |_archive_name| Ok(None),
    )
}

pub(super) fn prepare_apply_from_entries<TReadBytes, TSourceForEntry>(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    entry_names: &[String],
    apply_mappings: &BundleApplyMappings,
    apply_source: PreparedApplySource,
    mut read_entry_bytes: TReadBytes,
    mut source_for_entry: TSourceForEntry,
) -> AppResult<PreparedBundleApply>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
    TSourceForEntry: FnMut(&str) -> AppResult<Option<String>>,
{
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

    build_prepared_apply(
        plan_path,
        installation,
        manifest,
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        planned_entries,
        apply_source,
        &mut read_entry_bytes,
        &mut source_for_entry,
    )
}

fn build_prepared_apply<TReadBytes, TSourceForEntry>(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    discovered_accounts: Vec<LocalWowAccount>,
    selected_target_accounts: Vec<String>,
    character_mappings: Vec<CharacterMapping>,
    planned_entries: Vec<PlannedEntry>,
    apply_source: PreparedApplySource,
    read_entry_bytes: &mut TReadBytes,
    source_for_entry: &mut TSourceForEntry,
) -> AppResult<PreparedBundleApply>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
    TSourceForEntry: FnMut(&str) -> AppResult<Option<String>>,
{
    let rewrite_options = LuaRewriteOptions {
        rewrite_profile_keys: manifest.mapping.rewrite_profile_keys,
        rewrite_identity_strings: manifest.mapping.rewrite_identity_strings,
    };
    let cleanup_operations = build_cleanup_operations(&planned_entries, &manifest, installation)?;
    let cleanup_roots = cleanup_operations
        .iter()
        .map(|operation| operation.destination.clone())
        .collect::<Vec<_>>();
    let mut execution_operations = Vec::new();
    let mut summary = ApplyPlanSummary::default();

    for cleanup in cleanup_operations {
        summary.paths_to_remove += 1;
        execution_operations.push(PreparedApplyOperation::from_cleanup(cleanup));
    }

    for entry in &planned_entries {
        let policy = resource_policy_for_group(&manifest, entry.group);
        let preserve = policy == ResourceApplyPolicy::Preserve;
        let share = policy == ResourceApplyPolicy::Share;
        let cleanup_root = cleanup_scope_for_entry(entry, installation)?;
        let will_cleanup = cleanup_root
            .as_ref()
            .is_some_and(|root| cleanup_roots.iter().any(|candidate| candidate == root));

        let mut rewrite_applied = false;
        let action = if preserve {
            summary.files_to_preserve += 1;
            ApplyAction::Preserve
        } else if share && entry.destination.exists() {
            summary.files_to_preserve += 1;
            ApplyAction::Preserve
        } else {
            let source_bytes = read_entry_bytes(&entry.archive_name)?;
            let rewritten_bytes = preview_lua_bytes_rewrite(
                Path::new(&entry.archive_name),
                &source_bytes,
                &entry.rewrites,
                rewrite_options,
            )?;
            rewrite_applied = rewritten_bytes.is_some();

            if will_cleanup || !entry.destination.exists() {
                summary.files_to_add += 1;
                ApplyAction::Add
            } else if rewritten_bytes.as_deref().map_or_else(
                || file_contents_equal_to_bytes(&source_bytes, &entry.destination),
                |bytes| file_contents_equal_to_bytes(bytes, &entry.destination),
            )? {
                summary.files_to_skip += 1;
                ApplyAction::Skip
            } else {
                summary.files_to_replace += 1;
                ApplyAction::Replace
            }
        };

        if rewrite_applied {
            summary.files_to_rewrite += 1;
        }

        let source_path = match action {
            ApplyAction::Add | ApplyAction::Replace => source_for_entry(&entry.archive_name)?,
            ApplyAction::Skip | ApplyAction::Preserve => None,
            ApplyAction::Remove => unreachable!("planned bundle entry cannot produce remove"),
        };

        execution_operations.push(PreparedApplyOperation::from_entry(
            entry,
            action,
            rewrite_applied,
            source_path,
        ));
    }

    execution_operations.sort_by(|left, right| {
        apply_action_order(left.action)
            .cmp(&apply_action_order(right.action))
            .then_with(|| apply_group_order(left.group).cmp(&apply_group_order(right.group)))
            .then_with(|| left.destination.cmp(&right.destination))
            .then_with(|| left.archive_name.cmp(&right.archive_name))
    });
    let operations = execution_operations
        .iter()
        .map(PreparedApplyOperation::preview)
        .collect::<Vec<_>>();

    Ok(PreparedBundleApply {
        source: apply_source,
        plan: BundleApplyPlan {
            bundle_path: plan_path.to_path_buf(),
            target_flavor_root: installation.flavor_root.clone(),
            discovered_accounts,
            selected_target_accounts,
            character_mappings,
            operations,
            summary,
            helper_strategy: HelperStrategy::NativeRust,
            group_policies: ApplyGroupPolicies {
                addons: GroupPolicy {
                    policy: manifest.apply.addons,
                },
                wtf_common: GroupPolicy {
                    policy: manifest.apply.wtf_common,
                },
                wtf_characters: GroupPolicy {
                    policy: manifest.apply.wtf_characters,
                },
                fonts: GroupPolicy {
                    policy: manifest.apply.fonts,
                },
                interface_assets: GroupPolicy {
                    policy: manifest.apply.interface_assets,
                },
                metadata: GroupPolicy {
                    policy: ResourceApplyPolicy::Merge,
                },
            },
            manifest,
        },
        execution_operations,
    })
}
