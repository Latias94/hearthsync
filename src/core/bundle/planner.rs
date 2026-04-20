use std::path::Path;

use super::*;

mod model;
mod preview;

use model::{
    LogicalBundleApply, LogicalEntryDisposition, LogicalEntryOperation, ResolvedPreviewApply,
};
use preview::{build_pending_preview_apply, finalize_pending_preview_apply};

pub fn plan_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<BundleApplyPlan> {
    let source = PreparedApplySource::BundleArchive {
        bundle_path: bundle_path.to_path_buf(),
    };
    let manifest = source.bundle_manifest()?;

    plan_apply_from_source(bundle_path, installation, manifest, apply_mappings, &source)
}

pub(super) fn prepare_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<PreparedBundleApply> {
    let source = PreparedApplySource::BundleArchive {
        bundle_path: bundle_path.to_path_buf(),
    };
    let manifest = source.bundle_manifest()?;

    prepare_apply_from_source(bundle_path, installation, manifest, apply_mappings, source)
}

pub(super) fn plan_apply_from_source(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    apply_mappings: &BundleApplyMappings,
    source: &PreparedApplySource,
) -> AppResult<BundleApplyPlan> {
    let entry_names = source.logical_entry_names()?;
    let mut reader = source.open_reader()?;

    plan_apply_from_entry_reader(
        plan_path,
        installation,
        manifest,
        &entry_names,
        apply_mappings,
        |archive_name| source.read_logical_entry_bytes(&mut reader, archive_name),
    )
}

pub(super) fn prepare_apply_from_source(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    apply_mappings: &BundleApplyMappings,
    apply_source: PreparedApplySource,
) -> AppResult<PreparedBundleApply> {
    let read_source = apply_source.clone();
    let entry_names = read_source.logical_entry_names()?;
    let mut reader = read_source.open_reader()?;

    prepare_apply_from_entry_reader(
        plan_path,
        installation,
        manifest,
        &entry_names,
        apply_mappings,
        apply_source,
        |archive_name| read_source.read_logical_entry_bytes(&mut reader, archive_name),
    )
}

fn prepare_apply_from_entry_reader<TReadBytes>(
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

fn plan_apply_from_entry_reader<TReadBytes>(
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

#[cfg(test)]
pub(super) fn plan_apply_from_entries_with_reader<TReadBytes>(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    entry_names: &[String],
    apply_mappings: &BundleApplyMappings,
    read_entry_bytes: TReadBytes,
) -> AppResult<BundleApplyPlan>
where
    TReadBytes: FnMut(&str) -> AppResult<Vec<u8>>,
{
    plan_apply_from_entry_reader(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
        read_entry_bytes,
    )
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
