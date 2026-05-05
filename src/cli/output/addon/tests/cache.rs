use super::*;

#[test]
fn render_addon_cache_purge_reports_configured_summary() {
    let rendered = render_addon_cache_purge(&AddonCachePurgeResult {
        configured: true,
        cache_dir: Some(PathBuf::from("cache/addons")),
        removed_file_count: 3,
        removed_directory_count: 2,
        reclaimed_bytes: 2048,
    });

    assert!(rendered.contains("Purged addon cache: cache/addons"));
    assert!(rendered.contains("Removed files: 3"));
    assert!(rendered.contains("Removed directories: 2"));
    assert!(rendered.contains("Reclaimed bytes: 2048"));
}

#[test]
fn render_addon_cache_repair_reports_not_configured() {
    let rendered = render_addon_cache_repair(&AddonCacheRepairResult {
        configured: false,
        cache_dir: None,
        remote_policy: AddonCacheRepairRemotePolicyValue::ValidateRemote,
        scanned_metadata_count: 0,
        repaired_entry_count: 0,
        invalid_metadata_count: 0,
        missing_archive_count: 0,
        mismatched_archive_count: 0,
        orphan_archive_count: 0,
        partial_download_count: 0,
        remote_verified_entry_count: 0,
        remote_refreshed_entry_count: 0,
        remote_skipped_entry_count: 0,
        remote_check_failed_count: 0,
        expired_freshness_entry_count: 0,
        removed_file_count: 0,
        removed_directory_count: 0,
        reclaimed_bytes: 0,
    });

    assert_eq!(rendered, "Addon download cache is not configured.");
}

#[test]
fn render_addon_cache_repair_reports_remote_validation_summary() {
    let rendered = render_addon_cache_repair(&AddonCacheRepairResult {
        configured: true,
        cache_dir: Some(PathBuf::from("cache/addons")),
        remote_policy: AddonCacheRepairRemotePolicyValue::RequireRemote,
        scanned_metadata_count: 5,
        repaired_entry_count: 2,
        invalid_metadata_count: 1,
        missing_archive_count: 0,
        mismatched_archive_count: 1,
        orphan_archive_count: 0,
        partial_download_count: 1,
        remote_verified_entry_count: 3,
        remote_refreshed_entry_count: 1,
        remote_skipped_entry_count: 4,
        remote_check_failed_count: 2,
        expired_freshness_entry_count: 1,
        removed_file_count: 4,
        removed_directory_count: 2,
        reclaimed_bytes: 4096,
    });

    assert!(rendered.contains("Repaired addon cache: cache/addons"));
    assert!(rendered.contains("Remote repair policy: require_remote"));
    assert!(rendered.contains("Remote verified entries: 3"));
    assert!(rendered.contains("Remote refreshed entries: 1"));
    assert!(rendered.contains("Remote skipped entries: 4"));
    assert!(rendered.contains("Remote check failures: 2"));
    assert!(rendered.contains("Expired freshness entries: 1"));
}
