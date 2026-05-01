use super::*;

#[test]
fn addon_service_search_returns_app_owned_catalog() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let service =
        AddonService::with_runtime(AppRuntime::with_addon_provider(FakeSearchAddonProvider));

    let results = service
        .search(SearchAddonsRequest {
            installation,
            query: "weak".to_string(),
            limit: 5,
        })
        .expect("search addons");

    assert_eq!(results.query, "weak");
    assert_eq!(results.result_count, 1);
    assert_eq!(results.results[0].provider, "fake-provider");
    assert_eq!(results.results[0].source_label, "curseforge:42");
    assert_eq!(
        results.results[0].source.dependency_resolution_capability,
        AddonDependencyResolutionCapabilityValue::Unsupported
    );
}

#[test]
fn addon_service_purge_cache_reports_not_configured() {
    let service = AddonService::new();

    let result = service.purge_cache().expect("purge cache");

    assert!(!result.configured);
    assert_eq!(result.cache_dir, None);
    assert_eq!(result.removed_file_count, 0);
}

#[test]
fn addon_service_repair_cache_projects_provider_summary() {
    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    fs::create_dir_all(cache_dir.join("http").join("abc123")).expect("cache namespace");
    fs::write(
        cache_dir.join("http").join("abc123").join("addon.zip"),
        b"archive",
    )
    .expect("orphan archive");

    let service = AddonService::with_runtime(
        AppRuntime::with_addon_provider_options(AddonProviderOptionsValue {
            download_cache_dir: Some(cache_dir.clone()),
            retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
            http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue::ReuseWithinWindow {
                max_age_secs: 900,
            },
        })
        .expect("runtime"),
    );

    let result = service.repair_cache().expect("repair cache");

    assert!(result.configured);
    assert_eq!(result.cache_dir, Some(cache_dir));
    assert_eq!(result.repaired_entry_count, 1);
    assert_eq!(result.orphan_archive_count, 1);
    assert_eq!(result.removed_file_count, 1);
    assert!(
        !temp
            .path()
            .join("cache")
            .join("http")
            .join("abc123")
            .join("addon.zip")
            .exists()
    );
}
