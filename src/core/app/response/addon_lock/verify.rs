use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::lock::{
    AddonLockPackageDirectoryIssue as DomainAddonLockPackageDirectoryIssue,
    AddonLockVerifyResult as DomainAddonLockVerifyResult,
};

use super::super::super::map_owned_vec;
use super::diff::AddonLockDiffResult;

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
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonLockVerifyResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let untracked_addon_count = value.untracked_addons.len();
        let missing_package_count = value.missing_addon_directories.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            tracked_package_count: value.tracked_package_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            missing_package_count,
            missing_addon_directories: map_owned_vec(
                value.missing_addon_directories,
                AddonLockPackageDirectoryIssueResult::from_domain,
            ),
            diff: AddonLockDiffResult::from_domain_with_provider(value.diff, provider),
            matches: value.matches,
        }
    }
}
