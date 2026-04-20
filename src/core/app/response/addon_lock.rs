use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::lock::{
    AddonLockApplyResult as DomainAddonLockApplyResult,
    AddonLockDiffResult as DomainAddonLockDiffResult,
    AddonLockFieldChange as DomainAddonLockFieldChange, AddonLockInspection, AddonLockPackage,
    AddonLockPackageDiff as DomainAddonLockPackageDiff,
    AddonLockPackageDirectoryIssue as DomainAddonLockPackageDirectoryIssue,
    AddonLockPackageSnapshot as DomainAddonLockPackageSnapshot,
    AddonLockPlanResult as DomainAddonLockPlanResult,
    AddonLockSyncAction as DomainAddonLockSyncAction, AddonLockSyncActionKind,
    AddonLockVerifyResult as DomainAddonLockVerifyResult,
    AddonLockWriteResult as DomainAddonLockWriteResult,
};

use super::addon::{AddonSourceResult, TrackedAddonResult};
use super::map_domain_vec;

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageResult {
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_directories: Vec<String>,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
}

impl AddonLockPackageResult {
    pub(crate) fn from_domain(value: AddonLockPackage) -> Self {
        let source = AddonSourceResult::from_domain(value.source);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            name: value.name,
            version: value.version,
            source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            content_sha256: value.content_sha256,
            installed_at: value.installed_at,
            updated_at: value.updated_at,
            addon_directories: value.addon_directories,
            addon_count,
            addons: map_domain_vec(value.addons, TrackedAddonResult::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockInspectionResult {
    pub lock_path: PathBuf,
    pub generated_at: String,
    pub package_count: usize,
    pub packages: Vec<AddonLockPackageResult>,
}

impl AddonLockInspectionResult {
    pub(crate) fn from_domain(value: AddonLockInspection) -> Self {
        Self {
            lock_path: value.lock_path,
            generated_at: value.lock.generated_at,
            package_count: value.package_count,
            packages: map_domain_vec(value.lock.packages, AddonLockPackageResult::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockWriteResult {
    pub lock_path: PathBuf,
    pub package_count: usize,
    pub removed: bool,
}

impl AddonLockWriteResult {
    pub(crate) fn from_domain(value: DomainAddonLockWriteResult) -> Self {
        Self {
            lock_path: value.lock_path,
            package_count: value.package_count,
            removed: value.removed,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageSnapshotResult {
    pub comparison_key: String,
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: Option<String>,
    pub addon_directories: Vec<String>,
}

impl AddonLockPackageSnapshotResult {
    pub(crate) fn from_domain(value: DomainAddonLockPackageSnapshot) -> Self {
        let source = AddonSourceResult::from_domain(value.source);
        let source_label = source.display_name.clone();

        Self {
            comparison_key: value.comparison_key,
            package_id: value.package_id,
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            name: value.name,
            version: value.version,
            source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            content_sha256: value.content_sha256,
            addon_directories: value.addon_directories,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockFieldChangeResult {
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

impl AddonLockFieldChangeResult {
    pub(crate) fn from_domain(value: DomainAddonLockFieldChange) -> Self {
        Self {
            field: value.field,
            left: value.left,
            right: value.right,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDiffResult {
    pub comparison_key: String,
    pub left: AddonLockPackageSnapshotResult,
    pub right: AddonLockPackageSnapshotResult,
    pub changes: Vec<AddonLockFieldChangeResult>,
}

impl AddonLockPackageDiffResult {
    pub(crate) fn from_domain(value: DomainAddonLockPackageDiff) -> Self {
        Self {
            comparison_key: value.comparison_key,
            left: AddonLockPackageSnapshotResult::from_domain(value.left),
            right: AddonLockPackageSnapshotResult::from_domain(value.right),
            changes: map_domain_vec(value.changes, AddonLockFieldChangeResult::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockDiffResult {
    pub left_label: String,
    pub right_label: String,
    pub left_package_count: usize,
    pub right_package_count: usize,
    pub identical: bool,
    pub unchanged_packages: usize,
    pub added_package_count: usize,
    pub removed_package_count: usize,
    pub changed_package_count: usize,
    pub added_packages: Vec<AddonLockPackageSnapshotResult>,
    pub removed_packages: Vec<AddonLockPackageSnapshotResult>,
    pub changed_packages: Vec<AddonLockPackageDiffResult>,
}

impl AddonLockDiffResult {
    pub(crate) fn from_domain(value: DomainAddonLockDiffResult) -> Self {
        let added_package_count = value.added_packages.len();
        let removed_package_count = value.removed_packages.len();
        let changed_package_count = value.changed_packages.len();

        Self {
            left_label: value.left_label,
            right_label: value.right_label,
            left_package_count: value.left_package_count,
            right_package_count: value.right_package_count,
            identical: value.identical,
            unchanged_packages: value.unchanged_packages,
            added_package_count,
            removed_package_count,
            changed_package_count,
            added_packages: map_domain_vec(
                value.added_packages,
                AddonLockPackageSnapshotResult::from_domain,
            ),
            removed_packages: map_domain_vec(
                value.removed_packages,
                AddonLockPackageSnapshotResult::from_domain,
            ),
            changed_packages: map_domain_vec(
                value.changed_packages,
                AddonLockPackageDiffResult::from_domain,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDirectoryIssueResult {
    pub comparison_key: String,
    pub package_id: String,
    pub missing_addon_directories: Vec<String>,
}

impl AddonLockPackageDirectoryIssueResult {
    pub(crate) fn from_domain(value: DomainAddonLockPackageDirectoryIssue) -> Self {
        Self {
            comparison_key: value.comparison_key,
            package_id: value.package_id,
            missing_addon_directories: value.missing_addon_directories,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockVerifyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub tracked_package_count: usize,
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub missing_package_count: usize,
    pub missing_addon_directories: Vec<AddonLockPackageDirectoryIssueResult>,
    pub diff: AddonLockDiffResult,
    pub matches: bool,
}

impl AddonLockVerifyResult {
    pub(crate) fn from_domain(value: DomainAddonLockVerifyResult) -> Self {
        let untracked_addon_count = value.untracked_addons.len();
        let missing_package_count = value.missing_addon_directories.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            tracked_package_count: value.tracked_package_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            missing_package_count,
            missing_addon_directories: map_domain_vec(
                value.missing_addon_directories,
                AddonLockPackageDirectoryIssueResult::from_domain,
            ),
            diff: AddonLockDiffResult::from_domain(value.diff),
            matches: value.matches,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockSyncActionResult {
    pub kind: AddonLockSyncActionKind,
    pub comparison_key: String,
    pub package_id: String,
    pub name: Option<String>,
    pub addon_directories: Vec<String>,
    pub source: Option<AddonSourceResult>,
    pub source_label: Option<String>,
    pub reasons: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub requires_replace_existing: bool,
}

impl AddonLockSyncActionResult {
    pub(crate) fn from_domain(value: DomainAddonLockSyncAction) -> Self {
        let source = value.source.map(AddonSourceResult::from_domain);
        let source_label = source.as_ref().map(|source| source.display_name.clone());

        Self {
            kind: value.kind,
            comparison_key: value.comparison_key,
            package_id: value.package_id,
            name: value.name,
            addon_directories: value.addon_directories,
            source,
            source_label,
            reasons: value.reasons,
            blocked_reasons: value.blocked_reasons,
            requires_replace_existing: value.requires_replace_existing,
        }
    }
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
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub action_count: usize,
    pub actions: Vec<AddonLockSyncActionResult>,
}

impl AddonLockPlanResult {
    pub(crate) fn from_domain(value: DomainAddonLockPlanResult) -> Self {
        let untracked_addon_count = value.untracked_addons.len();
        let action_count = value.actions.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            install_count: value.install_count,
            update_count: value.update_count,
            remove_count: value.remove_count,
            metadata_only_count: value.metadata_only_count,
            unchanged_count: value.unchanged_count,
            blocked_count: value.blocked_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            action_count,
            actions: map_domain_vec(value.actions, AddonLockSyncActionResult::from_domain),
        }
    }
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
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub action_count: usize,
    pub actions: Vec<AddonLockSyncActionResult>,
    pub verification: AddonLockVerifyResult,
}

impl AddonLockApplyResult {
    pub(crate) fn from_domain(value: DomainAddonLockApplyResult) -> Self {
        let untracked_addon_count = value.untracked_addons.len();
        let action_count = value.actions.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            install_count: value.install_count,
            update_count: value.update_count,
            remove_count: value.remove_count,
            metadata_only_count: value.metadata_only_count,
            unchanged_count: value.unchanged_count,
            blocked_count: value.blocked_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            action_count,
            actions: map_domain_vec(value.actions, AddonLockSyncActionResult::from_domain),
            verification: AddonLockVerifyResult::from_domain(value.verification),
        }
    }
}
