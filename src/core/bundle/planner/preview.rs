use std::path::{Path, PathBuf};

use super::super::apply_model::planned::PlannedEntry;
use super::super::apply_model::prepared::{
    PreparedApplyOperation, PreparedApplySource, PreparedBundleApply,
};
use super::super::apply_model::preview::PreviewOperation;
use super::super::apply_policy::order::{apply_action_order, apply_group_order};
use super::super::execution::compare::file_contents_equal_to_bytes;
use super::super::types::{
    ApplyAction, ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary, BundleApplyPlan, GroupPolicy,
};
use super::model::{
    LogicalBundleApply, LogicalEntryDisposition, PendingExistingTargetPreviewEntry,
    PendingPreviewApply, ResolvedPreviewApply,
};
use crate::core::error::AppResult;
use crate::core::install::LocalWowAccount;
use crate::core::lua_patch::{CharacterMapping, LuaRewriteOptions, preview_lua_bytes_rewrite};
use crate::core::manifest::{BundleManifest, ResourceApplyPolicy};

pub(super) fn build_pending_preview_apply(
    logical_apply: LogicalBundleApply,
) -> PendingPreviewApply {
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

pub(super) fn finalize_pending_preview_apply<TReadBytes>(
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

    pub(super) fn into_plan(self) -> BundleApplyPlan {
        self.plan
    }

    pub(super) fn into_prepared_apply(
        self,
        apply_source: PreparedApplySource,
    ) -> PreparedBundleApply {
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
