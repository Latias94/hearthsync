use serde::Serialize;

use crate::core::app::{
    AddonIndexInspectionResult, AddonIndexInstallResult, AddonIndexUpdateResult,
    AddonInventoryResult, AddonLockApplyResult, AddonLockDiffResult, AddonLockInspectionResult,
    AddonLockPlanResult, AddonLockVerifyResult, AddonLockWriteResult, AddonSearchCatalogResult,
    BackupCatalogResult, BundleAddonLockApplyResult, BundleAddonLockPlanResult,
    BundleApplyPlanResult, BundleApplyResult, BundleInspectionResult, CreatedBackupResult,
    CreatedBundleResult, ExternalPackageAnalysisResult, ExternalPackageApplyPlanResult,
    ExternalPackageApplyResult, InstallationHealthResult, InstallationInspectionResult,
    InstallationScanResult, InstalledAddonPackageResult, RemovedAddonPackageResult,
    RestoredBackupResult, UpdatedAddonPackageResult,
};
#[cfg(test)]
use crate::core::app::{ExternalPackageSummaryResult, ExternalPackageWarningResult};
use crate::core::error::AppResult;

mod addon;
mod addon_lock;
mod backup;
mod bundle;
mod external_package;
mod shared;
mod system;
#[cfg(test)]
mod test_support;

pub(super) fn render_addon_index_inspection(item: &AddonIndexInspectionResult) -> String {
    addon::render_addon_index_inspection(item)
}

pub(super) fn render_addon_index_install(item: &AddonIndexInstallResult) -> String {
    addon::render_addon_index_install(item)
}

pub(super) fn render_addon_index_update(item: &AddonIndexUpdateResult) -> String {
    addon::render_addon_index_update(item)
}

pub(super) fn render_addon_search_catalog(item: &AddonSearchCatalogResult) -> String {
    addon::render_addon_search_catalog(item)
}

pub(super) fn render_addon_inventory(item: &AddonInventoryResult) -> String {
    addon::render_addon_inventory(item)
}

pub(super) fn render_addon_install(item: &InstalledAddonPackageResult) -> String {
    addon::render_addon_install(item)
}

pub(super) fn render_addon_update(item: &UpdatedAddonPackageResult) -> String {
    addon::render_addon_update(item)
}

pub(super) fn render_addon_remove(item: &RemovedAddonPackageResult) -> String {
    addon::render_addon_remove(item)
}

pub(super) fn render_bundle_archive_created(item: &CreatedBundleResult) -> String {
    bundle::render_bundle_archive_created(item)
}

pub(super) fn render_bundle_archive_inspection(item: &BundleInspectionResult) -> String {
    bundle::render_bundle_archive_inspection(item)
}

pub(super) fn render_bundle_apply_plan(item: &BundleApplyPlanResult) -> String {
    bundle::render_bundle_apply_plan(item)
}

pub(super) fn render_bundle_apply(item: &BundleApplyResult) -> String {
    bundle::render_bundle_apply(item)
}

pub(super) fn render_external_package_analysis(item: &ExternalPackageAnalysisResult) -> String {
    external_package::render_external_package_analysis(item)
}

pub(super) fn render_external_package_plan(item: &ExternalPackageApplyPlanResult) -> String {
    external_package::render_external_package_plan(item)
}

pub(super) fn render_external_package_apply(item: &ExternalPackageApplyResult) -> String {
    external_package::render_external_package_apply(item)
}

pub(super) fn render_installation_scan(item: &InstallationScanResult) -> String {
    system::render_installation_scan(item)
}

pub(super) fn render_installation_inspection(item: &InstallationInspectionResult) -> String {
    system::render_installation_inspection(item)
}

pub(super) fn render_installation_health_report(health: &InstallationHealthResult) -> String {
    system::render_installation_health_report(health)
}

pub(super) fn render_backup_created(item: &CreatedBackupResult) -> String {
    backup::render_backup_created(item)
}

pub(super) fn render_backup_catalog(item: &BackupCatalogResult) -> String {
    backup::render_backup_catalog(item)
}

pub(super) fn render_backup_restored(item: &RestoredBackupResult) -> String {
    backup::render_backup_restored(item)
}

pub(super) fn render_addon_lock_plan(item: &AddonLockPlanResult) -> String {
    addon_lock::render_addon_lock_plan(item)
}

pub(super) fn render_addon_lock_apply(item: &AddonLockApplyResult) -> String {
    addon_lock::render_addon_lock_apply(item)
}

pub(super) fn render_bundle_addon_lock_plan(item: &BundleAddonLockPlanResult) -> String {
    addon_lock::render_bundle_addon_lock_plan(item)
}

pub(super) fn render_bundle_addon_lock_apply(item: &BundleAddonLockApplyResult) -> String {
    addon_lock::render_bundle_addon_lock_apply(item)
}

pub(super) fn render_addon_lock_inspection(item: &AddonLockInspectionResult) -> String {
    addon_lock::render_addon_lock_inspection(item)
}

pub(super) fn render_addon_lock_write(item: &AddonLockWriteResult) -> String {
    addon_lock::render_addon_lock_write(item)
}

pub(super) fn render_addon_lock_diff(item: &AddonLockDiffResult) -> String {
    addon_lock::render_addon_lock_diff(item)
}

pub(super) fn render_addon_lock_verify(item: &AddonLockVerifyResult) -> String {
    addon_lock::render_addon_lock_verify(item)
}

#[cfg(test)]
pub(super) fn format_external_package_warnings(
    warnings: &[ExternalPackageWarningResult],
    summary: &ExternalPackageSummaryResult,
) -> String {
    shared::format_external_package_warnings(warnings, summary)
}

pub(super) fn render<T, F>(json: bool, value: &T, text_renderer: F) -> AppResult<()>
where
    T: Serialize,
    F: FnOnce(&T) -> String,
{
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", text_renderer(value));
    }

    Ok(())
}
