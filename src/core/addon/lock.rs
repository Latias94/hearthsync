mod apply;
mod apply_execute;
mod apply_model;
mod apply_prepare;
mod plan;
mod plan_actions;
mod plan_model;
mod plan_support;
mod source_resolution;
mod storage;
#[cfg(test)]
mod tests;
mod verify;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::addon::{AddonSourceRef, TrackedAddon};
use crate::core::install::DetectedFlavorInstallation;

pub(crate) use self::apply::apply_addon_lock_sync_task_with_provider;
pub use self::apply::{apply_addon_lock_sync, apply_addon_lock_sync_task};
pub use self::plan::{plan_addon_lock_sync, plan_addon_lock_sync_with_source_overrides};
pub(crate) use self::storage::sync_addon_lock_from_registry;
pub use self::storage::{inspect_addon_lock, lock_path, write_addon_lock};
pub use self::verify::{diff_addon_locks, verify_addon_lock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonLock {
    pub schema_version: u32,
    pub generated_at: String,
    pub packages: Vec<AddonLockPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonLockPackage {
    pub package_id: String,
    #[serde(default)]
    pub index_name: Option<String>,
    #[serde(default)]
    pub index_package_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub source: AddonSourceRef,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    pub content_sha256: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_directories: Vec<String>,
    pub addons: Vec<TrackedAddon>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockInspection {
    pub lock_path: PathBuf,
    pub lock: AddonLock,
    pub package_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockWriteResult {
    pub lock_path: PathBuf,
    pub package_count: usize,
    pub removed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageSnapshot {
    pub comparison_key: String,
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceRef,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: Option<String>,
    pub addon_directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockFieldChange {
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDiff {
    pub comparison_key: String,
    pub left: AddonLockPackageSnapshot,
    pub right: AddonLockPackageSnapshot,
    pub changes: Vec<AddonLockFieldChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockDiffResult {
    pub left_label: String,
    pub right_label: String,
    pub left_package_count: usize,
    pub right_package_count: usize,
    pub identical: bool,
    pub unchanged_packages: usize,
    pub added_packages: Vec<AddonLockPackageSnapshot>,
    pub removed_packages: Vec<AddonLockPackageSnapshot>,
    pub changed_packages: Vec<AddonLockPackageDiff>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockVerifyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub tracked_package_count: usize,
    pub untracked_addons: Vec<String>,
    pub missing_addon_directories: Vec<AddonLockPackageDirectoryIssue>,
    pub diff: AddonLockDiffResult,
    pub matches: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDirectoryIssue {
    pub comparison_key: String,
    pub package_id: String,
    pub missing_addon_directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AddonLockSyncActionKind {
    Install,
    Update,
    Remove,
    MetadataOnly,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockSyncAction {
    pub kind: AddonLockSyncActionKind,
    pub comparison_key: String,
    pub package_id: String,
    pub name: Option<String>,
    pub addon_directories: Vec<String>,
    pub source: Option<AddonSourceRef>,
    pub reasons: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub requires_replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPlanResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addons: Vec<String>,
    pub actions: Vec<AddonLockSyncAction>,
}

#[derive(Debug, Clone)]
pub struct AddonLockApplyRequest {
    pub installation: DetectedFlavorInstallation,
    pub lock_path: Option<PathBuf>,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub source_overrides: Vec<AddonLockSourceOverride>,
}

#[derive(Debug, Clone)]
pub struct AddonLockSourceOverride {
    pub comparison_key: String,
    pub archive_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockApplyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addons: Vec<String>,
    pub actions: Vec<AddonLockSyncAction>,
    pub verification: AddonLockVerifyResult,
}

fn comparison_key(
    package_id: &str,
    index_name: Option<&str>,
    index_package_id: Option<&str>,
    addon_directories: &[String],
) -> String {
    let index_name = index_name.map(str::trim).filter(|value| !value.is_empty());
    let index_package_id = index_package_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (index_name, index_package_id) {
        (Some(index_name), Some(index_package_id)) => {
            format!("index:{index_name}:{index_package_id}")
        }
        (None, Some(index_package_id)) => format!("index:{index_package_id}"),
        _ => {
            let mut normalized = addon_directories
                .iter()
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>();
            normalized.sort();
            normalized.dedup();
            if normalized.is_empty() {
                format!("package:{package_id}")
            } else {
                format!("addons:{}", normalized.join("+"))
            }
        }
    }
}

pub(crate) fn addon_lock_package_comparison_key(package: &AddonLockPackage) -> String {
    comparison_key(
        &package.package_id,
        package.index_name.as_deref(),
        package.index_package_id.as_deref(),
        &package.addon_directories,
    )
}

fn left_label(path: &Path) -> String {
    path.display().to_string()
}
