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

#[derive(Debug)]
struct PendingPreviewApply {
    plan_path: PathBuf,
    target_flavor_root: PathBuf,
    discovered_accounts: Vec<LocalWowAccount>,
    selected_target_accounts: Vec<String>,
    character_mappings: Vec<CharacterMapping>,
    manifest: BundleManifest,
    settled_operations: Vec<PreviewOperation>,
    pending_existing_target_entries: Vec<PendingExistingTargetPreviewEntry>,
}

#[derive(Debug)]
struct PendingExistingTargetPreviewEntry {
    entry: PlannedEntry,
}

#[derive(Debug)]
struct ResolvedPreviewApply {
    plan: BundleApplyPlan,
    preview_operations: Vec<PreviewOperation>,
}

pub fn plan_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<BundleApplyPlan> {
    let inspection = inspect_bundle(bundle_path)?;
    let entry_names = collect_bundle_entry_names(bundle_path)?;
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;

    plan_apply_from_entries_with_reader(
        bundle_path,
        installation,
        inspection.manifest,
        &entry_names,
        apply_mappings,
        |archive_name| read_bundle_entry_bytes_from_archive(&mut archive, archive_name),
    )
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
    )
}

pub(super) fn prepare_apply_from_entries<TReadBytes>(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    entry_names: &[String],
    apply_mappings: &BundleApplyMappings,
    apply_source: PreparedApplySource,
    mut read_entry_bytes: TReadBytes,
) -> AppResult<PreparedBundleApply>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    let resolved_preview_apply = resolve_preview_apply_from_entries(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
        &mut read_entry_bytes,
    )?;

    Ok(resolved_preview_apply.into_prepared_apply(apply_source))
}

pub(super) fn plan_apply_from_entries_with_reader<TReadBytes>(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    entry_names: &[String],
    apply_mappings: &BundleApplyMappings,
    mut read_entry_bytes: TReadBytes,
) -> AppResult<BundleApplyPlan>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    let resolved_preview_apply = resolve_preview_apply_from_entries(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
        &mut read_entry_bytes,
    )?;

    Ok(resolved_preview_apply.into_plan())
}

fn resolve_preview_apply_from_entries<TReadBytes>(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    entry_names: &[String],
    apply_mappings: &BundleApplyMappings,
    read_entry_bytes: &mut TReadBytes,
) -> AppResult<ResolvedPreviewApply>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    let logical_apply = plan_apply_from_entries(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
    )?;
    let pending_preview_apply = build_pending_preview_apply(logical_apply);

    finalize_pending_preview_apply(pending_preview_apply, read_entry_bytes)
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

fn build_pending_preview_apply(logical_apply: LogicalBundleApply) -> PendingPreviewApply {
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
    let mut settled_operations = Vec::new();
    let mut pending_existing_target_entries = Vec::new();

    for cleanup in cleanup_operations {
        settled_operations.push(PreviewOperation::from_cleanup(cleanup));
    }

    for entry_operation in entry_operations {
        let entry = entry_operation.entry;
        match entry_operation.disposition {
            LogicalEntryDisposition::Preserve => {
                settled_operations
                    .push(PreviewOperation::from_entry(&entry, ApplyAction::Preserve));
            }
            LogicalEntryDisposition::Materialize { will_cleanup } => {
                if will_cleanup || !entry.destination.exists() {
                    settled_operations.push(PreviewOperation::from_entry(&entry, ApplyAction::Add));
                } else {
                    pending_existing_target_entries
                        .push(PendingExistingTargetPreviewEntry { entry });
                }
            }
        }
    }

    PendingPreviewApply {
        plan_path,
        target_flavor_root,
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        manifest,
        settled_operations,
        pending_existing_target_entries,
    }
}

fn finalize_pending_preview_apply<TReadBytes>(
    pending_preview_apply: PendingPreviewApply,
    read_entry_bytes: &mut TReadBytes,
) -> AppResult<ResolvedPreviewApply>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    let PendingPreviewApply {
        plan_path,
        target_flavor_root,
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        manifest,
        settled_operations,
        pending_existing_target_entries,
    } = pending_preview_apply;
    let rewrite_options = LuaRewriteOptions {
        rewrite_profile_keys: manifest.mapping.rewrite_profile_keys,
        rewrite_identity_strings: manifest.mapping.rewrite_identity_strings,
    };
    let preview_operations = finalize_preview_operations(
        settled_operations,
        pending_existing_target_entries,
        rewrite_options,
        read_entry_bytes,
    )?;
    Ok(ResolvedPreviewApply::new(
        plan_path,
        target_flavor_root,
        discovered_accounts,
        selected_target_accounts,
        character_mappings,
        manifest,
        preview_operations,
    ))
}

fn finalize_preview_operations<TReadBytes>(
    mut settled_operations: Vec<PreviewOperation>,
    pending_existing_target_entries: Vec<PendingExistingTargetPreviewEntry>,
    rewrite_options: LuaRewriteOptions,
    read_entry_bytes: &mut TReadBytes,
) -> AppResult<Vec<PreviewOperation>>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    settled_operations.extend(finalize_existing_target_preview_entries(
        pending_existing_target_entries,
        rewrite_options,
        read_entry_bytes,
    )?);

    settled_operations.sort_by(|left, right| {
        apply_action_order(left.action())
            .cmp(&apply_action_order(right.action()))
            .then_with(|| apply_group_order(left.group()).cmp(&apply_group_order(right.group())))
            .then_with(|| left.destination().cmp(right.destination()))
            .then_with(|| left.archive_name().cmp(right.archive_name()))
    });

    Ok(settled_operations)
}

fn finalize_existing_target_preview_entries<TReadBytes>(
    pending_entries: Vec<PendingExistingTargetPreviewEntry>,
    rewrite_options: LuaRewriteOptions,
    read_entry_bytes: &mut TReadBytes,
) -> AppResult<Vec<PreviewOperation>>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    let mut finalized_operations = Vec::with_capacity(pending_entries.len());

    for pending_entry in pending_entries {
        let action = finalize_existing_target_action(
            &pending_entry.entry,
            rewrite_options,
            read_entry_bytes,
        )?;
        finalized_operations.push(PreviewOperation::from_entry(&pending_entry.entry, action));
    }

    Ok(finalized_operations)
}

fn finalize_existing_target_action<TReadBytes>(
    entry: &PlannedEntry,
    rewrite_options: LuaRewriteOptions,
    read_entry_bytes: &mut TReadBytes,
) -> AppResult<ApplyAction>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    let source_bytes = read_entry_bytes(&entry.archive_name)?;
    let comparison_bytes = if entry.rewrites.is_empty() {
        None
    } else {
        preview_lua_bytes_rewrite(
            Path::new(&entry.archive_name),
            &source_bytes,
            &entry.rewrites,
            rewrite_options,
        )?
    };

    if comparison_bytes.as_deref().map_or_else(
        || file_contents_equal_to_bytes(&source_bytes, &entry.destination),
        |bytes| file_contents_equal_to_bytes(bytes, &entry.destination),
    )? {
        Ok(ApplyAction::Skip)
    } else {
        Ok(ApplyAction::Replace)
    }
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

impl ResolvedPreviewApply {
    fn new(
        plan_path: PathBuf,
        target_flavor_root: PathBuf,
        discovered_accounts: Vec<LocalWowAccount>,
        selected_target_accounts: Vec<String>,
        character_mappings: Vec<CharacterMapping>,
        manifest: BundleManifest,
        preview_operations: Vec<PreviewOperation>,
    ) -> Self {
        let operations = preview_operations
            .iter()
            .cloned()
            .map(ApplyOperation::from)
            .collect::<Vec<_>>();
        let summary = ApplyPlanSummary::from_operations(&operations);
        let plan = build_bundle_apply_plan(
            plan_path,
            target_flavor_root,
            discovered_accounts,
            selected_target_accounts,
            character_mappings,
            operations,
            summary,
            manifest,
        );

        Self {
            plan,
            preview_operations,
        }
    }

    fn into_plan(self) -> BundleApplyPlan {
        self.plan
    }

    fn into_prepared_apply(self, apply_source: PreparedApplySource) -> PreparedBundleApply {
        PreparedBundleApply {
            source: apply_source,
            plan: self.plan,
            execution_operations: self
                .preview_operations
                .into_iter()
                .map(PreparedApplyOperation::from_preview)
                .collect(),
        }
    }
}
