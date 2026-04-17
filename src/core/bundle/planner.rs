use std::fs::File;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::*;

#[derive(Debug)]
struct LogicalBundleApply {
    plan_path: PathBuf,
    target_flavor_root: PathBuf,
    discovered_accounts: Vec<LocalWowAccount>,
    selected_target_accounts: Vec<String>,
    character_mappings: Vec<CharacterMapping>,
    manifest: BundleManifest,
    cleanup_operations: Vec<PlannedCleanup>,
    entry_operations: Vec<LogicalEntryOperation>,
}

#[derive(Debug)]
struct LogicalEntryOperation {
    entry: PlannedEntry,
    disposition: LogicalEntryDisposition,
}

#[derive(Debug, Clone, Copy)]
enum LogicalEntryDisposition {
    Preserve,
    Materialize { will_cleanup: bool },
}

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
    let logical_apply = plan_apply_from_entries(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
    )?;

    prepare_logical_apply(
        logical_apply,
        apply_source,
        &mut read_entry_bytes,
        &mut source_for_entry,
    )
}

fn plan_apply_from_entries(
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

fn prepare_logical_apply<TReadBytes, TSourceForEntry>(
    logical_apply: LogicalBundleApply,
    apply_source: PreparedApplySource,
    read_entry_bytes: &mut TReadBytes,
    source_for_entry: &mut TSourceForEntry,
) -> AppResult<PreparedBundleApply>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
    TSourceForEntry: FnMut(&str) -> AppResult<Option<String>>,
{
    let LogicalBundleApply {
        plan_path,
        target_flavor_root,
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        manifest,
        cleanup_operations,
        entry_operations,
    } = logical_apply;
    let rewrite_options = LuaRewriteOptions {
        rewrite_profile_keys: manifest.mapping.rewrite_profile_keys,
        rewrite_identity_strings: manifest.mapping.rewrite_identity_strings,
    };
    let mut execution_operations = Vec::new();
    let mut summary = ApplyPlanSummary::default();

    for cleanup in cleanup_operations {
        summary.paths_to_remove += 1;
        execution_operations.push(PreparedApplyOperation::from_cleanup(cleanup));
    }

    for entry_operation in &entry_operations {
        let entry = &entry_operation.entry;
        let (action, rewrite_applied, source_path) = match entry_operation.disposition {
            LogicalEntryDisposition::Preserve => {
                summary.files_to_preserve += 1;
                (ApplyAction::Preserve, false, None)
            }
            LogicalEntryDisposition::Materialize { will_cleanup } => {
                let source_bytes = read_entry_bytes(&entry.archive_name)?;
                let rewritten_bytes = preview_lua_bytes_rewrite(
                    Path::new(&entry.archive_name),
                    &source_bytes,
                    &entry.rewrites,
                    rewrite_options,
                )?;
                let rewrite_applied = rewritten_bytes.is_some();
                let action = if will_cleanup || !entry.destination.exists() {
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
                };

                let source_path = match action {
                    ApplyAction::Add | ApplyAction::Replace => {
                        source_for_entry(&entry.archive_name)?
                    }
                    ApplyAction::Skip | ApplyAction::Preserve => None,
                    ApplyAction::Remove => {
                        unreachable!("logical entry operation cannot finalize as remove")
                    }
                };

                (action, rewrite_applied, source_path)
            }
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
        plan: build_bundle_apply_plan(
            plan_path,
            target_flavor_root,
            discovered_accounts,
            selected_target_accounts,
            character_mappings,
            operations,
            summary,
            manifest,
        ),
        execution_operations,
    })
}

fn build_bundle_apply_plan(
    plan_path: PathBuf,
    target_flavor_root: PathBuf,
    discovered_accounts: Vec<LocalWowAccount>,
    selected_target_accounts: Vec<String>,
    character_mappings: Vec<CharacterMapping>,
    operations: Vec<ApplyOperation>,
    summary: ApplyPlanSummary,
    manifest: BundleManifest,
) -> BundleApplyPlan {
    BundleApplyPlan {
        bundle_path: plan_path,
        target_flavor_root,
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        operations,
        summary,
        helper_strategy: HelperStrategy::NativeRust,
        group_policies: build_group_policies(&manifest),
        manifest,
    }
}

fn build_group_policies(manifest: &BundleManifest) -> ApplyGroupPolicies {
    ApplyGroupPolicies {
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
    }
}
