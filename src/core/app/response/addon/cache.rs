use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::{
    AddonDownloadCachePurgeResult as DomainAddonDownloadCachePurgeResult,
    AddonDownloadCacheRepairResult as DomainAddonDownloadCacheRepairResult,
};

#[derive(Debug, Clone, Serialize)]
pub struct AddonCachePurgeResult {
    pub configured: bool,
    pub cache_dir: Option<PathBuf>,
    pub removed_file_count: usize,
    pub removed_directory_count: usize,
    pub reclaimed_bytes: u64,
}

impl AddonCachePurgeResult {
    pub(crate) fn from_domain(value: DomainAddonDownloadCachePurgeResult) -> Self {
        Self {
            configured: value.cache_dir.is_some(),
            cache_dir: value.cache_dir,
            removed_file_count: value.removed_file_count,
            removed_directory_count: value.removed_directory_count,
            reclaimed_bytes: value.reclaimed_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonCacheRepairResult {
    pub configured: bool,
    pub cache_dir: Option<PathBuf>,
    pub scanned_metadata_count: usize,
    pub repaired_entry_count: usize,
    pub invalid_metadata_count: usize,
    pub missing_archive_count: usize,
    pub mismatched_archive_count: usize,
    pub orphan_archive_count: usize,
    pub partial_download_count: usize,
    pub remote_verified_entry_count: usize,
    pub remote_refreshed_entry_count: usize,
    pub remote_check_failed_count: usize,
    pub expired_freshness_entry_count: usize,
    pub removed_file_count: usize,
    pub removed_directory_count: usize,
    pub reclaimed_bytes: u64,
}

impl AddonCacheRepairResult {
    pub(crate) fn from_domain(value: DomainAddonDownloadCacheRepairResult) -> Self {
        Self {
            configured: value.cache_dir.is_some(),
            cache_dir: value.cache_dir,
            scanned_metadata_count: value.scanned_metadata_count,
            repaired_entry_count: value.repaired_entry_count,
            invalid_metadata_count: value.invalid_metadata_count,
            missing_archive_count: value.missing_archive_count,
            mismatched_archive_count: value.mismatched_archive_count,
            orphan_archive_count: value.orphan_archive_count,
            partial_download_count: value.partial_download_count,
            remote_verified_entry_count: value.remote_verified_entry_count,
            remote_refreshed_entry_count: value.remote_refreshed_entry_count,
            remote_check_failed_count: value.remote_check_failed_count,
            expired_freshness_entry_count: value.expired_freshness_entry_count,
            removed_file_count: value.removed_file_count,
            removed_directory_count: value.removed_directory_count,
            reclaimed_bytes: value.reclaimed_bytes,
        }
    }
}
