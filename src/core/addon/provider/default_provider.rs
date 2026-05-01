use std::path::PathBuf;

use super::cache::{
    AddonDownloadCachePurgeResult, AddonDownloadCacheRepairResult, HttpNoValidatorCachePolicy,
    purge_download_cache_dir, repair_download_cache_dir,
};
use super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpRequest, HttpResponse, ReqwestHttpClient,
};
use super::materialize::{materialize_source_input_impl, materialize_source_ref_impl};
use super::source_adapter::{resolve_source_dependencies_impl, search_addons_impl};
use super::{
    AddonDependencyResolutionCapability, AddonProvider, AddonSearchRequest, AddonSearchResult,
    AddonSourceRef, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
    MaterializedAddonSource, ResolveAddonDependenciesRequest, ResolvedAddonDependencies,
};
use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonProviderRetryPolicy {
    pub max_attempts: u32,
}

impl Default for AddonProviderRetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 1 }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddonProviderOptions {
    pub download_cache_dir: Option<PathBuf>,
    pub retry_policy: AddonProviderRetryPolicy,
    pub http_no_validator_cache_policy: HttpNoValidatorCachePolicy,
}

#[derive(Debug, Clone)]
pub struct DefaultAddonProvider<H = ReqwestHttpClient> {
    http_client: H,
    options: AddonProviderOptions,
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

    pub fn http_client(&self) -> &H {
        &self.http_client
    }

    pub fn options(&self) -> &AddonProviderOptions {
        &self.options
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
        materialize_source_input_impl(
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
        materialize_source_ref_impl(
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
        match source {
            AddonSourceRef::CurseForgeMod { .. } => {
                AddonDependencyResolutionCapability::missing_required_only()
            }
            _ => AddonDependencyResolutionCapability::Unsupported,
        }
    }

    fn resolve_addon_dependencies(
        &self,
        request: ResolveAddonDependenciesRequest<'_>,
    ) -> AppResult<ResolvedAddonDependencies> {
        let http_client = RetryingHttpClient::new(&self.http_client, &self.options.retry_policy);
        resolve_source_dependencies_impl(&http_client, request.source, request.context)
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

    fn search_addons(&self, request: AddonSearchRequest<'_>) -> AppResult<Vec<AddonSearchResult>> {
        let http_client = RetryingHttpClient::new(&self.http_client, &self.options.retry_policy);
        search_addons_impl(&http_client, request.query, request.flavor, request.limit)
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

    use super::super::AddonProviderContext;
    use super::super::http::HttpResponse;
    use super::*;

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
