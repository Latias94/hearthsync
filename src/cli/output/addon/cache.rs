use super::*;

pub(in crate::cli) fn render_addon_cache_purge(item: &AddonCachePurgeResult) -> String {
    if !item.configured {
        return "Addon download cache is not configured.".to_string();
    }

    format!(
        "Purged addon cache: {}\nRemoved files: {}\nRemoved directories: {}\nReclaimed bytes: {}",
        item.cache_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        item.removed_file_count,
        item.removed_directory_count,
        item.reclaimed_bytes
    )
}

pub(in crate::cli) fn render_addon_cache_repair(item: &AddonCacheRepairResult) -> String {
    if !item.configured {
        return "Addon download cache is not configured.".to_string();
    }

    format!(
        "Repaired addon cache: {}\nScanned metadata entries: {}\nRepaired entries: {}\nInvalid metadata: {}\nMissing archives: {}\nMismatched archives: {}\nOrphan archives: {}\nPartial downloads: {}\nRemote verified entries: {}\nRemote refreshed entries: {}\nRemote check failures: {}\nExpired freshness entries: {}\nRemoved files: {}\nRemoved directories: {}\nReclaimed bytes: {}",
        item.cache_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        item.scanned_metadata_count,
        item.repaired_entry_count,
        item.invalid_metadata_count,
        item.missing_archive_count,
        item.mismatched_archive_count,
        item.orphan_archive_count,
        item.partial_download_count,
        item.remote_verified_entry_count,
        item.remote_refreshed_entry_count,
        item.remote_check_failed_count,
        item.expired_freshness_entry_count,
        item.removed_file_count,
        item.removed_directory_count,
        item.reclaimed_bytes
    )
}
