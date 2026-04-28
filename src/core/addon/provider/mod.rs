mod cache;
mod curseforge;
mod github;
mod http;
mod materialize;
mod parse;
mod source;
mod source_adapter;
#[cfg(test)]
mod tests;
mod validation;

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub use self::cache::{
    AddonDownloadCachePurgeResult, AddonDownloadCacheRepairResult, HttpNoValidatorCachePolicy,
};
use self::cache::{purge_download_cache_dir, repair_download_cache_dir};
use self::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpRequest, ReqwestHttpClient,
};
use self::materialize::{materialize_source_input_impl, materialize_source_ref_impl};
pub use self::source::AddonSourceRef;
pub(crate) use self::source::{
    addon_source_input_is_local_archive, canonicalize_local_archive_path,
    validate_absolute_local_archive_source_path,
};
use self::source_adapter::{resolve_source_dependencies_impl, search_addons_impl};
use super::policy::AddonReleaseChannel;
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;
use crate::core::task::CancellationToken;

#[cfg(test)]
use self::cache::{
    CachedArchiveMetadata, cached_archive_metadata_path, guess_archive_name_from_url,
    resolve_archive_path, write_cached_archive_metadata,
};
#[cfg(test)]
use self::validation::RemoteArchiveValidators;

#[derive(Debug)]
pub struct MaterializedAddonSource {
    pub source_ref: AddonSourceRef,
    pub archive_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub struct MaterializeSourceInputRequest<'a> {
    pub source: &'a str,
    pub stage_root: &'a Path,
    pub context: AddonProviderContext<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct MaterializeSourceRefRequest<'a> {
    pub source: &'a AddonSourceRef,
    pub stage_root: &'a Path,
    pub context: AddonProviderContext<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolveAddonDependenciesRequest<'a> {
    pub source: &'a AddonSourceRef,
    pub context: AddonProviderContext<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonDependencyResolutionStrategy {
    MissingRequiredOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonDependencyResolutionCapability {
    Unsupported,
    Supported {
        strategy: AddonDependencyResolutionStrategy,
    },
}

impl AddonDependencyResolutionCapability {
    pub fn missing_required_only() -> Self {
        Self::Supported {
            strategy: AddonDependencyResolutionStrategy::MissingRequiredOnly,
        }
    }

    pub fn supported_strategy(self) -> Option<AddonDependencyResolutionStrategy> {
        match self {
            Self::Unsupported => None,
            Self::Supported { strategy } => Some(strategy),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAddonDependencies {
    pub strategy: AddonDependencyResolutionStrategy,
    pub dependencies: Vec<AddonSourceRef>,
}

impl ResolvedAddonDependencies {
    pub fn missing_required_only(dependencies: Vec<AddonSourceRef>) -> Self {
        Self {
            strategy: AddonDependencyResolutionStrategy::MissingRequiredOnly,
            dependencies,
        }
    }
}

pub trait AddonDownloadProgressObserver {
    fn on_download_progress(
        &self,
        source: &AddonSourceRef,
        archive_name: &str,
        bytes_current: u64,
        bytes_total: Option<u64>,
        bytes_per_second: Option<u64>,
    );
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AddonSourceResolutionPolicy {
    pub release_channel: Option<AddonReleaseChannel>,
    pub allow_prerelease: Option<bool>,
}

#[derive(Clone, Copy, Default)]
pub struct AddonProviderContext<'a> {
    pub target_flavor: Option<WowFlavor>,
    pub cancellation: Option<&'a dyn CancellationToken>,
    download_progress: Option<&'a dyn AddonDownloadProgressObserver>,
    resolution_policy: AddonSourceResolutionPolicy,
}

impl fmt::Debug for AddonProviderContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddonProviderContext")
            .field("target_flavor", &self.target_flavor)
            .field("has_cancellation", &self.cancellation.is_some())
            .field("has_download_progress", &self.download_progress.is_some())
            .field("resolution_policy", &self.resolution_policy)
            .finish()
    }
}

impl<'a> AddonProviderContext<'a> {
    pub fn new(
        target_flavor: Option<WowFlavor>,
        cancellation: Option<&'a dyn CancellationToken>,
    ) -> Self {
        Self {
            target_flavor,
            cancellation,
            download_progress: None,
            resolution_policy: AddonSourceResolutionPolicy::default(),
        }
    }

    pub(crate) fn with_download_progress(
        mut self,
        download_progress: Option<&'a dyn AddonDownloadProgressObserver>,
    ) -> Self {
        self.download_progress = download_progress;
        self
    }

    pub(crate) fn with_resolution_policy(
        mut self,
        resolution_policy: AddonSourceResolutionPolicy,
    ) -> Self {
        self.resolution_policy = resolution_policy;
        self
    }

    pub fn resolution_policy(&self) -> AddonSourceResolutionPolicy {
        self.resolution_policy
    }

    pub fn report_download_progress(
        &self,
        source: &AddonSourceRef,
        archive_name: &str,
        bytes_current: u64,
        bytes_total: Option<u64>,
        bytes_per_second: Option<u64>,
    ) {
        if let Some(observer) = self.download_progress {
            observer.on_download_progress(
                source,
                archive_name,
                bytes_current,
                bytes_total,
                bytes_per_second,
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddonSearchRequest<'a> {
    pub query: &'a str,
    pub flavor: WowFlavor,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchResult {
    pub provider: &'static str,
    pub name: String,
    pub summary: Option<String>,
    pub source: AddonSourceRef,
    pub install_hint: String,
    pub website_url: Option<String>,
    pub provider_project_id: Option<u32>,
    pub provider_file_id: Option<u32>,
    pub download_count: u64,
}

pub trait AddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource>;

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource>;

    fn dependency_resolution_capability(
        &self,
        _source: &AddonSourceRef,
    ) -> AddonDependencyResolutionCapability {
        AddonDependencyResolutionCapability::Unsupported
    }

    fn resolve_addon_dependencies(
        &self,
        _request: ResolveAddonDependenciesRequest<'_>,
    ) -> AppResult<ResolvedAddonDependencies> {
        Err(AppError::Validation(
            "addon dependency installation is not supported by this provider".to_string(),
        ))
    }

    fn purge_download_cache(&self) -> AppResult<AddonDownloadCachePurgeResult> {
        Err(AppError::Validation(
            "addon provider does not support download cache management".to_string(),
        ))
    }

    fn repair_download_cache(&self) -> AppResult<AddonDownloadCacheRepairResult> {
        Err(AppError::Validation(
            "addon provider does not support download cache management".to_string(),
        ))
    }

    fn search_addons(&self, request: AddonSearchRequest<'_>) -> AppResult<Vec<AddonSearchResult>>;
}

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
    fn get(&self, request: HttpRequest) -> AppResult<self::http::HttpResponse> {
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
