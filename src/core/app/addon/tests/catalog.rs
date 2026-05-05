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
            provider_id: None,
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
    assert_eq!(results.provider_id, None);
    assert_eq!(results.failure_count, 0);
    assert!(results.failures.is_empty());
}

#[test]
fn addon_service_search_projects_partial_provider_failures() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let service = AddonService::with_runtime(AppRuntime::with_addon_provider(
        FakePartialCatalogAddonProvider,
    ));

    let results = service
        .search(SearchAddonsRequest {
            installation,
            query: "weak".to_string(),
            limit: 5,
            provider_id: None,
        })
        .expect("search addons");

    assert_eq!(results.result_count, 1);
    assert_eq!(results.failure_count, 1);
    assert_eq!(results.results[0].provider, "working-provider");
    assert_eq!(results.failures[0].provider_id, "broken-provider");
    assert_eq!(results.failures[0].source_family, "broken_family");
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
            cache_repair_remote_policy: AddonCacheRepairRemotePolicyValue::LocalOnly,
        })
        .expect("runtime"),
    );

    let result = service.repair_cache().expect("repair cache");

    assert!(result.configured);
    assert_eq!(result.cache_dir, Some(cache_dir));
    assert_eq!(result.repaired_entry_count, 1);
    assert_eq!(result.orphan_archive_count, 1);
    assert_eq!(result.removed_file_count, 1);
    assert_eq!(
        result.remote_policy,
        AddonCacheRepairRemotePolicyValue::LocalOnly
    );
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

#[derive(Clone)]
struct FakePartialCatalogAddonProvider;

impl AddonProvider for FakePartialCatalogAddonProvider {
    fn materialize_source_input(
        &self,
        _request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Err(AppError::Validation(
            "catalog-only provider does not materialize sources".to_string(),
        ))
    }

    fn materialize_source_ref(
        &self,
        _request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Err(AppError::Validation(
            "catalog-only provider does not materialize sources".to_string(),
        ))
    }

    fn search_addon_catalog(
        &self,
        request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<AddonSearchProviderCatalog> {
        assert_eq!(request.query, "weak");
        assert_eq!(request.limit, 5);
        assert_eq!(request.provider_id, None);
        Ok(AddonSearchProviderCatalog {
            results: vec![AddonSearchResult {
                provider: "working-provider",
                name: "WeakAuras".to_string(),
                summary: None,
                source: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                install_hint: "curseforge:42".to_string(),
                website_url: None,
                provider_project_id: Some(42),
                provider_file_id: None,
                download_count: 100,
            }],
            failures: vec![AddonSearchProviderFailure {
                provider_id: "broken-provider".to_string(),
                provider_name: "Broken Provider".to_string(),
                source_family: "broken_family".to_string(),
                message: "fixture failure".to_string(),
            }],
        })
    }

    fn search_addons(
        &self,
        _request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        panic!("search_addons should not be called when catalog search is overridden")
    }
}
