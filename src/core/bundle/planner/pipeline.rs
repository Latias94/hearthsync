use std::path::Path;

use super::super::apply_model::prepared::{PreparedApplySource, PreparedBundleApply};
use super::super::types::apply::{BundleApplyMappings, BundleApplyPlan};
use super::logical::plan_apply_from_entries as build_logical_apply_from_entries;
use super::model::ResolvedPreviewApply;
use super::preview::{
    build_pending_preview_apply, finalize_pending_preview_apply,
    resolve_pending_preview_apply_logical,
};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::manifest::BundleManifest;

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

pub(in crate::core::bundle) fn prepare_bundle_apply(
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

pub(in crate::core::bundle) fn plan_apply_from_source(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    apply_mappings: &BundleApplyMappings,
    source: &PreparedApplySource,
) -> AppResult<BundleApplyPlan> {
    let entry_names = source.logical_entry_names()?;
    build_public_plan_from_entries(
        plan_path,
        installation,
        manifest,
        &entry_names,
        apply_mappings,
    )
}

pub(in crate::core::bundle) fn prepare_apply_from_source(
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

#[cfg(test)]
pub(in crate::core::bundle) fn plan_apply_from_entries_with_reader<TReadBytes>(
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
    let _ = read_entry_bytes;
    build_public_plan_from_entries(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
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
    let logical_apply = build_logical_apply_from_entries(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
    )?;
    let pending_preview_apply = build_pending_preview_apply(logical_apply);

    finalize_pending_preview_apply(pending_preview_apply, read_entry_bytes)
}

fn build_public_plan_from_entries(
    plan_path: &Path,
    installation: &DetectedFlavorInstallation,
    manifest: BundleManifest,
    entry_names: &[String],
    apply_mappings: &BundleApplyMappings,
) -> AppResult<BundleApplyPlan> {
    let logical_apply = build_logical_apply_from_entries(
        plan_path,
        installation,
        manifest,
        entry_names,
        apply_mappings,
    )?;
    let pending_preview_apply = build_pending_preview_apply(logical_apply);
    let resolved_preview_apply = resolve_pending_preview_apply_logical(pending_preview_apply);

    Ok(resolved_preview_apply.into_plan())
}
