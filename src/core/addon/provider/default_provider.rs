use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::cache::{
    AddonCacheRepairRemotePolicy, AddonDownloadCachePurgeResult, AddonDownloadCacheRepairResult,
    HttpNoValidatorCachePolicy, purge_download_cache_dir, repair_download_cache_dir,
};
use super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpRequest, HttpResponse, ReqwestHttpClient,
};
use super::registry::AddonProviderRegistry;
use super::{
    AddonDependencyResolutionCapability, AddonProvider, AddonProviderDescriptor,
    AddonSearchProviderCatalog, AddonSearchRequest, AddonSearchResult, AddonSourceRef,
    AppliedAddonSourcePolicy, ApplyAddonSourcePolicyRequest, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource, ResolveAddonDependenciesRequest,
    ResolvedAddonDependencies,
};
use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;

pub const DEFAULT_ADDON_SEARCH_CACHE_TTL_SECS: u64 = 300;
const ADDON_SEARCH_CACHE_MAX_ENTRIES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonProviderRetryPolicy {
    pub max_attempts: u32,
}

impl Default for AddonProviderRetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 1 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonProviderOptions {
    pub download_cache_dir: Option<PathBuf>,
    pub retry_policy: AddonProviderRetryPolicy,
    pub http_no_validator_cache_policy: HttpNoValidatorCachePolicy,
    pub cache_repair_remote_policy: AddonCacheRepairRemotePolicy,
    pub search_cache_ttl_secs: u64,
}

impl Default for AddonProviderOptions {
    fn default() -> Self {
        Self {
            download_cache_dir: None,
            retry_policy: AddonProviderRetryPolicy::default(),
            http_no_validator_cache_policy: HttpNoValidatorCachePolicy::default(),
            cache_repair_remote_policy: AddonCacheRepairRemotePolicy::default(),
            search_cache_ttl_secs: DEFAULT_ADDON_SEARCH_CACHE_TTL_SECS,
        }
    }
}

impl AddonProviderOptions {
    fn search_cache_ttl(&self) -> Option<Duration> {
        if self.search_cache_ttl_secs == 0 {
            None
        } else {
            Some(Duration::from_secs(self.search_cache_ttl_secs))
        }
    }
}

#[derive(Debug, Clone)]
pub struct DefaultAddonProvider<H = ReqwestHttpClient> {
    http_client: H,
    options: AddonProviderOptions,
    search_cache: Arc<Mutex<AddonSearchCache>>,
}

impl DefaultAddonProvider<ReqwestHttpClient> {
    pub fn new(http_client: ReqwestHttpClient) -> Self {
        Self::with_http_client(http_client)
    }
}

impl<H> DefaultAddonProvider<H> {
    pub fn with_http_client(http_client: H) -> Self {
        Self {
            http_client,
            options: AddonProviderOptions::default(),
            search_cache: Arc::new(Mutex::new(AddonSearchCache::default())),
        }
    }

    pub fn with_options(mut self, options: AddonProviderOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_download_cache_dir(mut self, download_cache_dir: Option<PathBuf>) -> Self {
        self.options.download_cache_dir = download_cache_dir;
        self
    }

    pub fn with_retry_policy(mut self, retry_policy: AddonProviderRetryPolicy) -> Self {
        self.options.retry_policy = retry_policy;
        self
    }

    pub fn with_http_no_validator_cache_policy(
        mut self,
        http_no_validator_cache_policy: HttpNoValidatorCachePolicy,
    ) -> Self {
        self.options.http_no_validator_cache_policy = http_no_validator_cache_policy;
        self
    }

    pub fn with_cache_repair_remote_policy(
        mut self,
        cache_repair_remote_policy: AddonCacheRepairRemotePolicy,
    ) -> Self {
        self.options.cache_repair_remote_policy = cache_repair_remote_policy;
        self
    }

    pub fn with_search_cache_ttl_secs(mut self, search_cache_ttl_secs: u64) -> Self {
        self.options.search_cache_ttl_secs = search_cache_ttl_secs;
        self
    }

    pub fn http_client(&self) -> &H {
        &self.http_client
    }

    pub fn options(&self) -> &AddonProviderOptions {
        &self.options
    }
}

#[derive(Debug, Default)]
struct AddonSearchCache {
    entries: HashMap<AddonSearchCacheKey, AddonSearchCacheEntry>,
}

impl AddonSearchCache {
    fn get(
        &mut self,
        key: &AddonSearchCacheKey,
        now: Instant,
        ttl: Duration,
    ) -> Option<AddonSearchProviderCatalog> {
        match self.entries.get(key) {
            Some(entry) if entry.is_fresh(now, ttl) => Some(entry.catalog.clone()),
            Some(_) => {
                self.entries.remove(key);
                None
            }
            None => None,
        }
    }

    fn insert(
        &mut self,
        key: AddonSearchCacheKey,
        catalog: AddonSearchProviderCatalog,
        now: Instant,
        ttl: Duration,
    ) {
        self.entries.retain(|_, entry| entry.is_fresh(now, ttl));
        if self.entries.len() >= ADDON_SEARCH_CACHE_MAX_ENTRIES {
            return;
        }

        self.entries.insert(
            key,
            AddonSearchCacheEntry {
                cached_at: now,
                catalog,
            },
        );
    }
}

#[derive(Debug, Clone)]
struct AddonSearchCacheEntry {
    cached_at: Instant,
    catalog: AddonSearchProviderCatalog,
}

impl AddonSearchCacheEntry {
    fn is_fresh(&self, now: Instant, ttl: Duration) -> bool {
        now.duration_since(self.cached_at) <= ttl
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AddonSearchCacheKey {
    provider_id: Option<String>,
    query: String,
    flavor: String,
    limit: usize,
}

impl AddonSearchCacheKey {
    fn from_request(request: AddonSearchRequest<'_>) -> Self {
        Self {
            provider_id: request.provider_id.map(str::to_string),
            query: request.query.to_string(),
            flavor: request.flavor.as_str().to_string(),
            limit: request.limit,
        }
    }
}

impl Default for DefaultAddonProvider<ReqwestHttpClient> {
    fn default() -> Self {
        Self::new(ReqwestHttpClient::default())
    }
}

#[derive(Debug, Clone, Copy)]
struct RetryingHttpClient<'a, H> {
    inner: &'a H,
    max_attempts: u32,
}

impl<'a, H> RetryingHttpClient<'a, H> {
    fn new(inner: &'a H, retry_policy: &AddonProviderRetryPolicy) -> Self {
        Self {
            inner,
            max_attempts: retry_policy.max_attempts.max(1),
        }
    }
}

impl<H> HttpClient for RetryingHttpClient<'_, H>
where
    H: HttpClient,
{
    fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
        retry_http(self.max_attempts, || self.inner.get(request.clone()))
    }

    fn download_to_path(
        &self,
        request: HttpDownloadRequest,
        cancellation: &dyn CancellationToken,
        observer: Option<&dyn HttpDownloadProgressObserver>,
    ) -> AppResult<HttpDownloadResponse> {
        retry_http(self.max_attempts, || {
            self.inner
                .download_to_path(request.clone(), cancellation, observer)
        })
    }
}

impl<H> AddonProvider for DefaultAddonProvider<H>
where
    H: HttpClient,
{
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        let http_client = RetryingHttpClient::new(&self.http_client, &self.options.retry_policy);
        AddonProviderRegistry::new().materialize_source_input(
            &http_client,
            request.source,
            request.stage_root,
            request.context,
            &self.options,
        )
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        let http_client = RetryingHttpClient::new(&self.http_client, &self.options.retry_policy);
        AddonProviderRegistry::new().materialize_source_ref(
            &http_client,
            request.source,
            request.stage_root,
            request.context,
            &self.options,
        )
    }

    fn dependency_resolution_capability(
        &self,
        source: &AddonSourceRef,
    ) -> AddonDependencyResolutionCapability {
        AddonProviderRegistry::new().dependency_resolution_capability(source)
    }

    fn provider_descriptors(&self) -> Vec<AddonProviderDescriptor> {
        AddonProviderRegistry::new().provider_descriptors()
    }

    fn apply_source_policy(
        &self,
        request: ApplyAddonSourcePolicyRequest<'_>,
    ) -> AppResult<AppliedAddonSourcePolicy> {
        AddonProviderRegistry::new().apply_source_policy(request)
    }

    fn resolve_addon_dependencies(
        &self,
        request: ResolveAddonDependenciesRequest<'_>,
    ) -> AppResult<ResolvedAddonDependencies> {
        let http_client = RetryingHttpClient::new(&self.http_client, &self.options.retry_policy);
        AddonProviderRegistry::new().resolve_addon_dependencies(&http_client, request)
    }

    fn purge_download_cache(&self) -> AppResult<AddonDownloadCachePurgeResult> {
        purge_download_cache_dir(self.options.download_cache_dir.as_deref())
    }

    fn repair_download_cache(&self) -> AppResult<AddonDownloadCacheRepairResult> {
        let http_client = RetryingHttpClient::new(&self.http_client, &self.options.retry_policy);
        repair_download_cache_dir(
            &http_client,
            self.options.download_cache_dir.as_deref(),
            &self.options,
        )
    }

    fn search_addon_catalog(
        &self,
        request: AddonSearchRequest<'_>,
    ) -> AppResult<AddonSearchProviderCatalog> {
        let cache_key = AddonSearchCacheKey::from_request(request);
        let now = Instant::now();
        let cache_ttl = self.options.search_cache_ttl();
        if let Some(ttl) = cache_ttl
            && let Some(cached_catalog) = self
                .search_cache
                .lock()
                .map_err(|_| {
                    AppError::Validation(
                        "addon provider search cache lock was poisoned".to_string(),
                    )
                })?
                .get(&cache_key, now, ttl)
        {
            return Ok(cached_catalog);
        }

        let http_client = RetryingHttpClient::new(&self.http_client, &self.options.retry_policy);
        let catalog = AddonProviderRegistry::new().search_addon_catalog(&http_client, request)?;
        if catalog.failures.is_empty()
            && let Some(ttl) = cache_ttl
        {
            self.search_cache
                .lock()
                .map_err(|_| {
                    AppError::Validation(
                        "addon provider search cache lock was poisoned".to_string(),
                    )
                })?
                .insert(cache_key, catalog.clone(), now, ttl);
        }

        Ok(catalog)
    }

    fn search_addons(&self, request: AddonSearchRequest<'_>) -> AppResult<Vec<AddonSearchResult>> {
        Ok(self.search_addon_catalog(request)?.results)
    }
}

fn retry_http<T>(max_attempts: u32, mut operation: impl FnMut() -> AppResult<T>) -> AppResult<T> {
    let mut last_error = None;
    for _ in 0..max_attempts.max(1) {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error @ AppError::Cancelled(_)) => return Err(error),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        AppError::Validation(
            "addon provider retry policy must allow at least one attempt".to_string(),
        )
    }))
}

#[cfg(test)]
mod default_provider_tests {
    use std::cell::{Cell, RefCell};

    use tempfile::tempdir;
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::super::http::HttpResponse;
    use super::super::{
        AddonProviderContext, AddonProviderSourceCapability, AddonSourceFamily,
        AddonSourceResolutionPolicy,
    };
    use super::*;
    use crate::core::addon::policy::{AddonPolicyPin, AddonReleaseChannel};
    use crate::core::install::WowFlavor;

    #[test]
    fn default_addon_provider_reports_dependency_resolution_capability_by_source_kind() {
        let provider = DefaultAddonProvider::with_http_client(ReqwestHttpClient::default());

        assert_eq!(
            provider.dependency_resolution_capability(&AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: None,
            }),
            AddonDependencyResolutionCapability::missing_required_only()
        );
        assert_eq!(
            provider.dependency_resolution_capability(&AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: None,
            }),
            AddonDependencyResolutionCapability::Unsupported
        );
    }

    #[test]
    fn default_addon_provider_rejects_unsupported_dependency_resolution_by_provider() {
        let provider = DefaultAddonProvider::with_http_client(ReqwestHttpClient::default());
        let source = AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: Some("addon.zip".to_string()),
        };

        let error = provider
            .resolve_addon_dependencies(ResolveAddonDependenciesRequest {
                source: &source,
                context: AddonProviderContext::default(),
            })
            .expect_err("github dependency resolution should fail before HTTP");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(error.to_string().contains("provider `github`"));
        assert!(error.to_string().contains("source family `github_release`"));
    }

    #[test]
    fn default_addon_provider_reports_source_capabilities() {
        let provider = DefaultAddonProvider::with_http_client(ReqwestHttpClient::default());
        let descriptors = provider.provider_descriptors();
        let capabilities = provider.source_capabilities();

        assert_eq!(descriptors.len(), 6);
        assert_eq!(capabilities.len(), 6);
        assert_eq!(
            capabilities,
            descriptors
                .iter()
                .copied()
                .map(AddonProviderDescriptor::source_capability)
                .collect::<Vec<_>>()
        );
        assert_provider_descriptor_ids_are_unique(&descriptors);

        let local = source_capability(&capabilities, AddonSourceFamily::LOCAL_ARCHIVE, "local");
        assert!(local.can_parse_input);
        assert!(local.can_materialize);
        assert!(!local.can_search);
        assert_eq!(
            local.dependency_resolution,
            AddonDependencyResolutionCapability::Unsupported
        );
        assert!(!local.supports_remote_cache_validators);

        let http = source_capability(&capabilities, AddonSourceFamily::HTTP_ARCHIVE, "http");
        assert_eq!(http.input_prefix, Some("http:// or https://"));
        assert!(http.can_parse_input);
        assert!(http.can_materialize);
        assert!(!http.can_search);
        assert!(http.supports_remote_cache_validators);

        let curseforge = source_capability(
            &capabilities,
            AddonSourceFamily::CURSEFORGE_MOD,
            "curseforge",
        );
        assert_eq!(curseforge.input_prefix, Some("curseforge:"));
        assert!(curseforge.can_parse_input);
        assert!(curseforge.can_materialize);
        assert!(curseforge.can_search);
        assert_eq!(
            curseforge.dependency_resolution,
            AddonDependencyResolutionCapability::missing_required_only()
        );
        assert!(curseforge.supports_release_channel);
        assert!(curseforge.supports_prerelease);
        assert!(!curseforge.supports_version_pin);
        assert!(curseforge.supports_file_id_pin);
        assert!(curseforge.supports_remote_cache_validators);

        let github = source_capability(&capabilities, AddonSourceFamily::GITHUB_RELEASE, "github");
        assert_eq!(github.input_prefix, Some("github:"));
        assert!(github.can_parse_input);
        assert!(github.can_materialize);
        assert!(!github.can_search);
        assert_eq!(
            github.dependency_resolution,
            AddonDependencyResolutionCapability::Unsupported
        );
        assert!(github.supports_release_channel);
        assert!(github.supports_prerelease);
        assert!(github.supports_version_pin);
        assert!(!github.supports_file_id_pin);
        assert!(github.supports_remote_cache_validators);

        let wago = source_capability(&capabilities, AddonSourceFamily::WAGO_ADDON, "wago");
        assert_eq!(wago.input_prefix, Some("wago:"));
        assert!(wago.can_parse_input);
        assert!(wago.can_materialize);
        assert!(!wago.can_search);
        assert_eq!(
            wago.dependency_resolution,
            AddonDependencyResolutionCapability::Unsupported
        );
        assert!(wago.supports_release_channel);
        assert!(wago.supports_prerelease);
        assert!(wago.supports_version_pin);
        assert!(!wago.supports_file_id_pin);
        assert!(!wago.supports_remote_cache_validators);

        let tukui = source_capability(&capabilities, AddonSourceFamily::TUKUI_ADDON, "tukui");
        assert_eq!(tukui.input_prefix, Some("tukui:"));
        assert!(tukui.can_parse_input);
        assert!(tukui.can_materialize);
        assert!(tukui.can_search);
        assert_eq!(
            tukui.dependency_resolution,
            AddonDependencyResolutionCapability::Unsupported
        );
        assert!(!tukui.supports_release_channel);
        assert!(!tukui.supports_prerelease);
        assert!(!tukui.supports_version_pin);
        assert!(!tukui.supports_file_id_pin);
        assert!(!tukui.supports_remote_cache_validators);
    }

    #[test]
    fn default_addon_provider_rejects_search_scoped_to_non_catalog_provider() {
        let provider = DefaultAddonProvider::with_http_client(ReqwestHttpClient::default());

        let error = provider
            .search_addon_catalog(AddonSearchRequest {
                query: "weak",
                flavor: WowFlavor::Retail,
                limit: 10,
                provider_id: Some("github"),
            })
            .expect_err("github catalog search should fail before HTTP");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error
                .to_string()
                .contains("does not support catalog search")
        );
    }

    #[test]
    fn default_addon_provider_caches_successful_catalog_searches() {
        #[derive(Default)]
        struct FakeHttpClient {
            requests: RefCell<Vec<HttpRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                self.requests.borrow_mut().push(request);
                Ok(HttpResponse {
                    status_code: 200,
                    body: format!("[{}]", tukui_catalog_addon_json("elvui", "ElvUI")),
                })
            }

            fn download_to_path(
                &self,
                _request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                panic!("download should not be called in this test")
            }
        }

        let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default());
        let request = AddonSearchRequest {
            query: "elv",
            flavor: WowFlavor::Retail,
            limit: 10,
            provider_id: Some("tukui"),
        };

        let first = provider
            .search_addon_catalog(request)
            .expect("first catalog search");
        let second = provider
            .search_addon_catalog(request)
            .expect("second catalog search should hit cache");

        assert_eq!(first.results.len(), 1);
        assert_eq!(second.results.len(), 1);
        assert_eq!(provider.http_client().requests.borrow().len(), 1);
    }

    #[test]
    fn default_addon_provider_can_disable_catalog_search_cache() {
        #[derive(Default)]
        struct FakeHttpClient {
            requests: RefCell<Vec<HttpRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                self.requests.borrow_mut().push(request);
                Ok(HttpResponse {
                    status_code: 200,
                    body: format!("[{}]", tukui_catalog_addon_json("elvui", "ElvUI")),
                })
            }

            fn download_to_path(
                &self,
                _request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                panic!("download should not be called in this test")
            }
        }

        let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
            .with_search_cache_ttl_secs(0);
        let request = AddonSearchRequest {
            query: "elv",
            flavor: WowFlavor::Retail,
            limit: 10,
            provider_id: Some("tukui"),
        };

        provider
            .search_addon_catalog(request)
            .expect("first catalog search");
        provider
            .search_addon_catalog(request)
            .expect("second catalog search should bypass cache");

        assert_eq!(provider.http_client().requests.borrow().len(), 2);
    }

    #[test]
    fn default_addon_provider_does_not_cache_failed_catalog_searches() {
        struct FakeHttpClient {
            responses: RefCell<Vec<AppResult<HttpResponse>>>,
            requests: RefCell<Vec<HttpRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                self.requests.borrow_mut().push(request);
                self.responses.borrow_mut().remove(0)
            }

            fn download_to_path(
                &self,
                _request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                panic!("download should not be called in this test")
            }
        }

        let provider = DefaultAddonProvider::with_http_client(FakeHttpClient {
            responses: RefCell::new(vec![
                Ok(HttpResponse {
                    status_code: 500,
                    body: String::new(),
                }),
                Ok(HttpResponse {
                    status_code: 200,
                    body: format!("[{}]", tukui_catalog_addon_json("elvui", "ElvUI")),
                }),
            ]),
            requests: RefCell::new(Vec::new()),
        });
        let request = AddonSearchRequest {
            query: "elv",
            flavor: WowFlavor::Retail,
            limit: 10,
            provider_id: Some("tukui"),
        };

        let error = provider
            .search_addon_catalog(request)
            .expect_err("first catalog search should fail");
        assert!(error.to_string().contains("Tukui catalog request failed"));

        let retry = provider
            .search_addon_catalog(request)
            .expect("failed search should not be cached");

        assert_eq!(retry.results.len(), 1);
        assert_eq!(provider.http_client().requests.borrow().len(), 2);
    }

    fn tukui_catalog_addon_json(slug: &str, name: &str) -> String {
        format!(
            r#"{{"slug":"{slug}","name":"{name}","url":"https://api.tukui.org/v1/download/{slug}/token","version":"1.0.0","patch":["12.0.1"],"web_url":"https://tukui.org/{slug}","small_desc":"A UI package"}}"#
        )
    }

    fn assert_provider_descriptor_ids_are_unique(descriptors: &[AddonProviderDescriptor]) {
        let mut ids = Vec::new();
        for descriptor in descriptors {
            assert!(!descriptor.provider_id.trim().is_empty());
            assert!(!descriptor.source_family.id().trim().is_empty());
            assert!(
                ids.iter().all(|id| *id != descriptor.provider_id),
                "provider id `{}` should be unique",
                descriptor.provider_id
            );
            ids.push(descriptor.provider_id);
        }
    }

    fn source_capability<'a>(
        capabilities: &'a [AddonProviderSourceCapability],
        source_family: AddonSourceFamily,
        provider_id: &str,
    ) -> &'a AddonProviderSourceCapability {
        capabilities
            .iter()
            .find(|capability| {
                capability.source_family == source_family && capability.provider_id == provider_id
            })
            .unwrap_or_else(|| {
                panic!(
                    "missing source capability `{provider_id}` for {:?}",
                    source_family
                )
            })
    }

    #[test]
    fn default_addon_provider_applies_source_policy_through_provider_capabilities() {
        let provider = DefaultAddonProvider::with_http_client(ReqwestHttpClient::default());
        let curseforge_source = AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: None,
        };
        let curseforge_file_pin = AddonPolicyPin::FileId { value: 777 };
        let release_policy = AddonSourceResolutionPolicy {
            release_channel: Some(AddonReleaseChannel::Alpha),
            allow_prerelease: Some(true),
        };

        let applied = provider
            .apply_source_policy(ApplyAddonSourcePolicyRequest {
                source: &curseforge_source,
                pin: Some(&curseforge_file_pin),
                resolution_policy: release_policy,
            })
            .expect("apply curseforge policy");

        assert_eq!(
            applied.source,
            AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: Some(777),
            }
        );
        assert_eq!(applied.resolution_policy, release_policy);

        let github_source = AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: Some("addon.zip".to_string()),
        };
        let github_version_pin = AddonPolicyPin::Version {
            value: "v2.0.0".to_string(),
        };

        let applied = provider
            .apply_source_policy(ApplyAddonSourcePolicyRequest {
                source: &github_source,
                pin: Some(&github_version_pin),
                resolution_policy: release_policy,
            })
            .expect("apply github policy");

        assert_eq!(
            applied.source,
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: Some("v2.0.0".to_string()),
                asset_name: Some("addon.zip".to_string()),
            }
        );
        assert_eq!(applied.resolution_policy, release_policy);

        let wago_source = AddonSourceRef::WagoAddon {
            project_id: "qv63A7Gb".to_string(),
            release_id: None,
        };
        let wago_version_pin = AddonPolicyPin::Version {
            value: "vdx1042w".to_string(),
        };

        let applied = provider
            .apply_source_policy(ApplyAddonSourcePolicyRequest {
                source: &wago_source,
                pin: Some(&wago_version_pin),
                resolution_policy: release_policy,
            })
            .expect("apply wago policy");

        assert_eq!(
            applied.source,
            AddonSourceRef::WagoAddon {
                project_id: "qv63A7Gb".to_string(),
                release_id: Some("vdx1042w".to_string()),
            }
        );
        assert_eq!(applied.resolution_policy, release_policy);
    }

    #[test]
    fn default_addon_provider_rejects_unsupported_source_policy_before_materialization() {
        let provider = DefaultAddonProvider::with_http_client(ReqwestHttpClient::default());
        let github_source = AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: Some("addon.zip".to_string()),
        };
        let file_pin = AddonPolicyPin::FileId { value: 777 };

        let error = provider
            .apply_source_policy(ApplyAddonSourcePolicyRequest {
                source: &github_source,
                pin: Some(&file_pin),
                resolution_policy: AddonSourceResolutionPolicy::default(),
            })
            .expect_err("github file id pin should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error
                .to_string()
                .contains("pinned file id is not supported")
        );
        assert!(error.to_string().contains("provider: github"));
        assert!(error.to_string().contains("capability: file id pin"));

        let local_source = AddonSourceRef::LocalArchive {
            path: std::path::PathBuf::from("C:\\addons\\WeakAuras.zip"),
        };
        let error = provider
            .apply_source_policy(ApplyAddonSourcePolicyRequest {
                source: &local_source,
                pin: None,
                resolution_policy: AddonSourceResolutionPolicy {
                    release_channel: Some(AddonReleaseChannel::Stable),
                    allow_prerelease: None,
                },
            })
            .expect_err("local release channel should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error
                .to_string()
                .contains("release channel is not supported")
        );
        assert!(error.to_string().contains("provider: local"));
        assert!(
            error
                .to_string()
                .contains("capability: release channel policy")
        );
    }

    #[test]
    fn default_addon_provider_accepts_injected_http_client() {
        #[derive(Default)]
        struct FakeHttpClient {
            requests: RefCell<Vec<HttpRequest>>,
            downloads: RefCell<Vec<HttpDownloadRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                self.requests.borrow_mut().push(request);
                Ok(HttpResponse {
                    status_code: 200,
                    body: r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/addon.zip"}]}"#.to_string(),
                })
            }

            fn download_to_path(
                &self,
                request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                self.downloads.borrow_mut().push(request.clone());
                let file = std::fs::File::create(&request.destination).expect("archive file");
                let mut zip = ZipWriter::new(file);
                zip.start_file(
                    "WeakAuras/WeakAuras.toc",
                    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
                )
                .expect("start zip entry");
                use std::io::Write;
                zip.write_all(b"## Interface: 110000\n## Version: 1.0.0\n")
                    .expect("write zip entry");
                zip.finish().expect("finish zip");
                Ok(HttpDownloadResponse {
                    status_code: 200,
                    headers: Vec::new(),
                })
            }
        }

        let temp = tempdir().expect("temp dir");
        let http_client = FakeHttpClient::default();
        let provider = DefaultAddonProvider::with_http_client(http_client)
            .with_download_cache_dir(Some(temp.path().join("cache")));

        let materialized = provider
            .materialize_source_ref(MaterializeSourceRefRequest {
                source: &AddonSourceRef::GitHubRelease {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    tag: Some("v1.2.3".to_string()),
                    asset_name: Some("addon.zip".to_string()),
                },
                stage_root: temp.path(),
                context: AddonProviderContext::default(),
            })
            .expect("materialize github source");

        assert!(materialized.archive_path.exists());
        assert_eq!(
            provider.options().download_cache_dir,
            Some(temp.path().join("cache"))
        );
        assert_eq!(provider.http_client().requests.borrow().len(), 1);
        assert_eq!(provider.http_client().downloads.borrow().len(), 1);
    }

    #[test]
    fn default_addon_provider_retries_failed_http_archive_downloads() {
        #[derive(Default)]
        struct FakeHttpClient {
            attempts: RefCell<usize>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
                panic!("get should not be called in this test")
            }

            fn download_to_path(
                &self,
                request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                let mut attempts = self.attempts.borrow_mut();
                *attempts += 1;
                if *attempts == 1 {
                    return Err(AppError::Validation(
                        "transient download failure".to_string(),
                    ));
                }

                std::fs::write(&request.destination, b"archive").expect("archive file");
                Ok(HttpDownloadResponse {
                    status_code: 200,
                    headers: Vec::new(),
                })
            }
        }

        let temp = tempdir().expect("temp dir");
        let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
            .with_retry_policy(AddonProviderRetryPolicy { max_attempts: 2 });
        let source = AddonSourceRef::HttpArchive {
            url: "https://example.com/addon.zip".to_string(),
        };

        let materialized = provider
            .materialize_source_ref(MaterializeSourceRefRequest {
                source: &source,
                stage_root: temp.path(),
                context: AddonProviderContext::default(),
            })
            .expect("materialize with retry");

        assert!(materialized.archive_path.exists());
        assert_eq!(*provider.http_client().attempts.borrow(), 2);
    }

    #[test]
    fn default_addon_provider_forwards_cancellation_without_retrying() {
        #[derive(Default)]
        struct FakeHttpClient {
            attempts: Cell<usize>,
            saw_cancelled: Cell<bool>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
                panic!("get should not be called in this test")
            }

            fn download_to_path(
                &self,
                _request: HttpDownloadRequest,
                cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                self.attempts.set(self.attempts.get() + 1);
                self.saw_cancelled.set(cancellation.is_cancelled());
                Err(AppError::Cancelled(
                    "addon provider download cancelled".to_string(),
                ))
            }
        }

        struct AlwaysCancelled;

        impl CancellationToken for AlwaysCancelled {
            fn is_cancelled(&self) -> bool {
                true
            }
        }

        let temp = tempdir().expect("temp dir");
        let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
            .with_retry_policy(AddonProviderRetryPolicy { max_attempts: 3 });
        let source = AddonSourceRef::HttpArchive {
            url: "https://example.com/addon.zip".to_string(),
        };
        let cancellation = AlwaysCancelled;

        let error = provider
            .materialize_source_ref(MaterializeSourceRefRequest {
                source: &source,
                stage_root: temp.path(),
                context: AddonProviderContext::new(None, Some(&cancellation)),
            })
            .expect_err("cancelled download");

        assert!(matches!(error, AppError::Cancelled(_)));
        assert_eq!(provider.http_client().attempts.get(), 1);
        assert!(provider.http_client().saw_cancelled.get());
    }
}
