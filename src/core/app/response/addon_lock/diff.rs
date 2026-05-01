use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::lock::{
    AddonLockDiffResult as DomainAddonLockDiffResult,
    AddonLockFieldChange as DomainAddonLockFieldChange,
    AddonLockPackageDiff as DomainAddonLockPackageDiff,
    AddonLockPackageSnapshot as DomainAddonLockPackageSnapshot,
};

use super::super::super::map_owned_vec;
use super::super::addon::AddonSourceResult;

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
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonLockPackageSnapshot,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
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
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonLockPackageDiff,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            comparison_key: value.comparison_key,
            left: AddonLockPackageSnapshotResult::from_domain_with_provider(value.left, provider),
            right: AddonLockPackageSnapshotResult::from_domain_with_provider(value.right, provider),
            changes: map_owned_vec(value.changes, AddonLockFieldChangeResult::from_domain),
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
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonLockDiffResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
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
            added_packages: map_owned_vec(value.added_packages, |value| {
                AddonLockPackageSnapshotResult::from_domain_with_provider(value, provider)
            }),
            removed_packages: map_owned_vec(value.removed_packages, |value| {
                AddonLockPackageSnapshotResult::from_domain_with_provider(value, provider)
            }),
            changed_packages: map_owned_vec(value.changed_packages, |value| {
                AddonLockPackageDiffResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}
