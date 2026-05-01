use std::path::PathBuf;

use serde::Serialize;

use crate::core::bundle::{
    BundleEntryCounts, BundleInspection, CreatedBundle as DomainCreatedBundle,
};

use super::manifest::{
    BundleManifestResult, BundlePackageResult, BundleResourcesResult, BundleSourceResult,
};

#[derive(Debug, Clone, Serialize)]
pub struct BundleEntryCountsResult {
    pub total_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub metadata: usize,
}

impl BundleEntryCountsResult {
    pub(crate) fn from_domain(value: BundleEntryCounts) -> Self {
        Self {
            total_files: value.total_files,
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters,
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            metadata: value.metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleInspectionResult {
    pub archive_path: PathBuf,
    pub package: BundlePackageResult,
    pub source: BundleSourceResult,
    pub resources: BundleResourcesResult,
    pub entries: BundleEntryCountsResult,
}

impl BundleInspectionResult {
    pub(crate) fn from_domain(value: BundleInspection) -> Self {
        let package = BundlePackageResult::from_domain(value.manifest.package);
        let source = BundleSourceResult::from_domain(value.manifest.source);
        let resources = BundleResourcesResult::from_domain(value.manifest.resources);

        Self {
            archive_path: value.archive_path,
            package,
            source,
            resources,
            entries: BundleEntryCountsResult::from_domain(value.entries),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBundleResult {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub manifest: BundleManifestResult,
}

impl CreatedBundleResult {
    pub(crate) fn from_domain(value: DomainCreatedBundle) -> Self {
        Self {
            archive_path: value.archive_path,
            archived_files: value.archived_files,
            manifest: BundleManifestResult::from_domain(value.manifest),
        }
    }
}
