use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::archive_read::addon_lock::extract_embedded_addon_lock;
use super::constants::ADDON_LOCK_ENTRY;
use super::types::{BundleAddonLockApply, BundleAddonLockApplyRequest, BundleAddonLockPlan};
use crate::core::addon::lock::{
    AddonLockApplyRequest, AddonLockSourceOverride, apply_addon_lock_sync,
    plan_addon_lock_sync_with_source_overrides,
};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

pub(super) struct ExtractedAddonLock {
    pub(super) lock_path: PathBuf,
    pub(super) source_overrides: Vec<AddonLockSourceOverride>,
    pub(super) _stage_dir: TempDir,
}

pub fn plan_bundle_addon_lock(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<BundleAddonLockPlan> {
    let extracted = extract_embedded_addon_lock(bundle_path)?;
    let mut plan = plan_addon_lock_sync_with_source_overrides(
        installation,
        Some(&extracted.lock_path),
        &extracted.source_overrides,
    )?;
    plan.lock_path = PathBuf::from(ADDON_LOCK_ENTRY);

    Ok(BundleAddonLockPlan {
        bundle_path: bundle_path.to_path_buf(),
        embedded_lock_entry: ADDON_LOCK_ENTRY.to_string(),
        plan,
    })
}

pub fn apply_bundle_addon_lock(
    request: BundleAddonLockApplyRequest,
) -> AppResult<BundleAddonLockApply> {
    let extracted = extract_embedded_addon_lock(&request.bundle_path)?;
    let mut apply = apply_addon_lock_sync(AddonLockApplyRequest {
        installation: request.installation,
        lock_path: Some(extracted.lock_path.clone()),
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        source_overrides: extracted.source_overrides,
    })?;
    apply.lock_path = PathBuf::from(ADDON_LOCK_ENTRY);
    apply.verification.lock_path = PathBuf::from(ADDON_LOCK_ENTRY);

    Ok(BundleAddonLockApply {
        bundle_path: request.bundle_path,
        embedded_lock_entry: ADDON_LOCK_ENTRY.to_string(),
        apply,
    })
}
