mod curseforge;
mod github;
mod http;
mod parse;
#[cfg(test)]
mod tests;

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use self::curseforge::{
    CurseForgeFile, CurseForgeFileDependency, CurseForgeFileReleaseType,
    resolve_curseforge_file_with_client, search_curseforge_mods_with_client,
};
use self::github::GitHubReleaseAsset;
use self::github::{
    fetch_github_release_with_client, fetch_github_releases_with_client, select_github_release,
    select_github_release_asset,
};
use self::http::{
    HttpClient, HttpDownloadProgress, HttpDownloadProgressObserver, HttpDownloadRequest,
    HttpDownloadResponse, HttpHeader, HttpRequest, ReqwestHttpClient,
};
use self::parse::{parse_curseforge_source, parse_github_source};
use super::policy::AddonReleaseChannel;
use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;
use crate::core::task::{CancellationToken, NeverCancel};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddonSourceRef {
    LocalArchive {
        path: PathBuf,
    },
    HttpArchive {
        url: String,
    },
    #[serde(rename = "curseforge_mod", alias = "curse_forge_mod")]
    CurseForgeMod {
        mod_id: u32,
        file_id: Option<u32>,
    },
    #[serde(rename = "github_release", alias = "git_hub_release")]
    GitHubRelease {
        owner: String,
        repo: String,
        tag: Option<String>,
        asset_name: Option<String>,
    },
}

impl AddonSourceRef {
    pub fn display_name(&self) -> String {
        match self {
            Self::LocalArchive { path } => path.display().to_string(),
            Self::HttpArchive { url } => url.clone(),
            Self::CurseForgeMod { mod_id, file_id } => {
                let mut text = format!("curseforge:{mod_id}");
                if let Some(file_id) = file_id {
                    text.push('@');
                    text.push_str(&file_id.to_string());
                }
                text
            }
            Self::GitHubRelease {
                owner,
                repo,
                tag,
                asset_name,
            } => {
                let mut text = format!("github:{owner}/{repo}");
                if let Some(tag) = tag {
                    text.push('@');
                    text.push_str(tag);
                }
                if let Some(asset_name) = asset_name {
                    text.push('#');
                    text.push_str(asset_name);
                }
                text
            }
        }
    }
}

#[derive(Debug)]
pub struct MaterializedAddonSource {
    pub source_ref: AddonSourceRef,
    pub archive_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonDownloadCachePurgeResult {
    pub cache_dir: Option<PathBuf>,
    pub removed_file_count: usize,
    pub removed_directory_count: usize,
    pub reclaimed_bytes: u64,
}

impl AddonDownloadCachePurgeResult {
    fn not_configured() -> Self {
        Self {
            cache_dir: None,
            removed_file_count: 0,
            removed_directory_count: 0,
            reclaimed_bytes: 0,
        }
    }

    fn for_cache_dir(cache_dir: PathBuf, stats: RemovedPathStats) -> Self {
        Self {
            cache_dir: Some(cache_dir),
            removed_file_count: stats.removed_file_count,
            removed_directory_count: stats.removed_directory_count,
            reclaimed_bytes: stats.reclaimed_bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonDownloadCacheRepairResult {
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

impl AddonDownloadCacheRepairResult {
    fn not_configured() -> Self {
        Self {
            cache_dir: None,
            scanned_metadata_count: 0,
            repaired_entry_count: 0,
            invalid_metadata_count: 0,
            missing_archive_count: 0,
            mismatched_archive_count: 0,
            orphan_archive_count: 0,
            partial_download_count: 0,
            remote_verified_entry_count: 0,
            remote_refreshed_entry_count: 0,
            remote_check_failed_count: 0,
            expired_freshness_entry_count: 0,
            removed_file_count: 0,
            removed_directory_count: 0,
            reclaimed_bytes: 0,
        }
    }

    fn for_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir: Some(cache_dir),
            ..Self::not_configured()
        }
    }
}

#[derive(Debug, Clone)]
struct ResolvedDownloadArtifact {
    cache_source_ref: AddonSourceRef,
    download_url: String,
    archive_name: String,
    headers: Vec<HttpHeader>,
    remote_validators: RemoteArchiveValidators,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CachedArchiveMetadata {
    source_display_name: String,
    #[serde(default)]
    source_ref: Option<AddonSourceRef>,
    archive_name: String,
    file_size: u64,
    file_sha256: String,
    #[serde(default)]
    fetched_at_unix_timestamp: Option<u64>,
    #[serde(default)]
    remote_validators: RemoteArchiveValidators,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct RemoteArchiveValidators {
    content_length: Option<u64>,
    last_modified: Option<String>,
    etag: Option<String>,
    sha256: Option<String>,
    sha1: Option<String>,
    md5: Option<String>,
}

impl RemoteArchiveValidators {
    fn is_empty(&self) -> bool {
        self.content_length.is_none()
            && self.last_modified.is_none()
            && self.etag.is_none()
            && self.sha256.is_none()
            && self.sha1.is_none()
            && self.md5.is_none()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct RemovedPathStats {
    removed_file_count: usize,
    removed_directory_count: usize,
    reclaimed_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheRemoteRepairStatus {
    Unchanged,
    Refreshed,
    Expired,
    Failed,
    Skipped,
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

pub(crate) fn canonicalize_local_archive_path(path: &Path) -> AppResult<PathBuf> {
    let resolved =
        fs::canonicalize(path).map_err(|_| AppError::NotFound(path.display().to_string()))?;
    if !resolved.is_file() {
        return Err(AppError::Validation(format!(
            "addon source must be a file archive: {}",
            resolved.display()
        )));
    }

    Ok(normalize_canonical_archive_path(resolved))
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

const DEFAULT_HTTP_NO_VALIDATOR_CACHE_WINDOW_SECS: u64 = 15 * 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpNoValidatorCachePolicy {
    AlwaysRefresh,
    ReuseWithinWindow { max_age_secs: u64 },
}

impl Default for HttpNoValidatorCachePolicy {
    fn default() -> Self {
        Self::ReuseWithinWindow {
            max_age_secs: DEFAULT_HTTP_NO_VALIDATOR_CACHE_WINDOW_SECS,
        }
    }
}

impl HttpNoValidatorCachePolicy {
    fn max_age_secs(&self) -> Option<u64> {
        match self {
            Self::AlwaysRefresh => None,
            Self::ReuseWithinWindow { max_age_secs } => Some(*max_age_secs),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonProviderOptions {
    pub download_cache_dir: Option<PathBuf>,
    pub retry_policy: AddonProviderRetryPolicy,
    pub http_no_validator_cache_policy: HttpNoValidatorCachePolicy,
}

impl Default for AddonProviderOptions {
    fn default() -> Self {
        Self {
            download_cache_dir: None,
            retry_policy: AddonProviderRetryPolicy::default(),
            http_no_validator_cache_policy: HttpNoValidatorCachePolicy::default(),
        }
    }
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

fn materialize_source_input_impl(
    http_client: &impl HttpClient,
    source: &str,
    stage_root: &Path,
    context: AddonProviderContext<'_>,
    options: &AddonProviderOptions,
) -> AppResult<MaterializedAddonSource> {
    if let Some(source_ref) = parse_curseforge_source(source)? {
        return materialize_source_ref_impl(http_client, &source_ref, stage_root, context, options);
    }

    if let Some(source_ref) = parse_github_source(source)? {
        return materialize_source_ref_impl(http_client, &source_ref, stage_root, context, options);
    }

    if source.starts_with("https://") || source.starts_with("http://") {
        let source_ref = AddonSourceRef::HttpArchive {
            url: source.to_string(),
        };
        return materialize_source_ref_impl(http_client, &source_ref, stage_root, context, options);
    }

    let path = canonicalize_local_archive_path(Path::new(source))?;

    Ok(MaterializedAddonSource {
        source_ref: AddonSourceRef::LocalArchive { path: path.clone() },
        archive_path: path,
    })
}

fn materialize_source_ref_impl(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    stage_root: &Path,
    context: AddonProviderContext<'_>,
    options: &AddonProviderOptions,
) -> AppResult<MaterializedAddonSource> {
    match source {
        AddonSourceRef::LocalArchive { path } => Ok(MaterializedAddonSource {
            source_ref: source.clone(),
            archive_path: path.clone(),
        }),
        AddonSourceRef::HttpArchive { url } => {
            let archive_path = materialize_http_archive(
                http_client,
                source,
                url,
                stage_root,
                context.cancellation,
                context.download_progress,
                options,
            )?;
            Ok(MaterializedAddonSource {
                source_ref: source.clone(),
                archive_path,
            })
        }
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => {
            let file = resolve_curseforge_file_with_client(
                http_client,
                *mod_id,
                *file_id,
                context.target_flavor,
                curseforge_release_type_limit(context.resolution_policy),
            )?;
            let download_url = file.download_url.clone().ok_or_else(|| {
                AppError::Validation(format!(
                    "CurseForge file `{}` does not provide a download URL",
                    file.id
                ))
            })?;
            let artifact = ResolvedDownloadArtifact {
                cache_source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: *mod_id,
                    file_id: Some(file.id),
                },
                download_url,
                archive_name: file.file_name.clone(),
                headers: Vec::new(),
                remote_validators: remote_validators_for_curseforge_file(&file),
            };
            let archive_path = materialize_downloaded_archive(
                http_client,
                artifact,
                stage_root,
                context.cancellation,
                context.download_progress,
                options,
            )?;
            Ok(MaterializedAddonSource {
                source_ref: source.clone(),
                archive_path,
            })
        }
        AddonSourceRef::GitHubRelease {
            owner,
            repo,
            tag,
            asset_name,
        } => {
            let release = match tag.as_deref() {
                Some(tag) => fetch_github_release_with_client(http_client, owner, repo, Some(tag))?,
                None if github_allows_prerelease(context.resolution_policy) => {
                    let releases = fetch_github_releases_with_client(http_client, owner, repo)?;
                    select_github_release(&releases, true)?.clone()
                }
                None => fetch_github_release_with_client(http_client, owner, repo, None)?,
            };
            let asset = select_github_release_asset(&release, asset_name.as_deref())?;
            let artifact = ResolvedDownloadArtifact {
                cache_source_ref: AddonSourceRef::GitHubRelease {
                    owner: owner.clone(),
                    repo: repo.clone(),
                    tag: Some(release.tag_name.clone()),
                    asset_name: Some(asset.name.clone()),
                },
                download_url: asset.browser_download_url.clone(),
                archive_name: asset.name.clone(),
                headers: Vec::new(),
                remote_validators: remote_validators_for_github_asset(asset),
            };
            let archive_path = materialize_downloaded_archive(
                http_client,
                artifact,
                stage_root,
                context.cancellation,
                context.download_progress,
                options,
            )?;
            Ok(MaterializedAddonSource {
                source_ref: source.clone(),
                archive_path,
            })
        }
    }
}

fn search_addons_impl(
    http_client: &impl HttpClient,
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<Vec<AddonSearchResult>> {
    search_curseforge_mods_with_client(http_client, query, flavor, limit)
}

fn resolve_source_dependencies_impl(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    context: AddonProviderContext<'_>,
) -> AppResult<ResolvedAddonDependencies> {
    match source {
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => {
            let file = resolve_curseforge_file_with_client(
                http_client,
                *mod_id,
                *file_id,
                context.target_flavor,
                curseforge_release_type_limit(context.resolution_policy),
            )?;
            Ok(ResolvedAddonDependencies::missing_required_only(
                required_dependency_sources_for_curseforge_file(*mod_id, &file.dependencies),
            ))
        }
        _ => Err(AppError::Validation(format!(
            "addon dependency installation is currently only supported for CurseForge sources, but `{}` uses `{}`",
            source.display_name(),
            source_kind_label(source),
        ))),
    }
}

fn materialize_downloaded_archive(
    http_client: &impl HttpClient,
    artifact: ResolvedDownloadArtifact,
    stage_root: &Path,
    cancellation: Option<&dyn CancellationToken>,
    download_progress: Option<&dyn AddonDownloadProgressObserver>,
    options: &AddonProviderOptions,
) -> AppResult<PathBuf> {
    let archive_path = resolve_archive_path(
        &artifact.cache_source_ref,
        &artifact.archive_name,
        stage_root,
        options,
    );
    if should_reuse_cached_archive(
        &artifact.cache_source_ref,
        &artifact.archive_name,
        &artifact.remote_validators,
        &archive_path,
        options,
    ) {
        return Ok(archive_path);
    }

    let provider_progress =
        download_progress.map(|observer| ForwardAddonDownloadProgressObserver {
            source: &artifact.cache_source_ref,
            archive_name: &artifact.archive_name,
            inner: observer,
        });
    download_to_path_with_headers(
        http_client,
        &artifact.download_url,
        artifact.headers,
        &archive_path,
        cancellation,
        provider_progress
            .as_ref()
            .map(|observer| observer as &dyn HttpDownloadProgressObserver),
    )?;
    write_cached_archive_metadata(
        &archive_path,
        &artifact.cache_source_ref,
        &artifact.archive_name,
        &artifact.remote_validators,
        options,
    )?;
    Ok(archive_path)
}

fn materialize_http_archive(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    url: &str,
    stage_root: &Path,
    cancellation: Option<&dyn CancellationToken>,
    download_progress: Option<&dyn AddonDownloadProgressObserver>,
    options: &AddonProviderOptions,
) -> AppResult<PathBuf> {
    let archive_name =
        guess_archive_name_from_url(url).unwrap_or_else(|| "downloaded-addon.zip".to_string());
    let archive_path = resolve_archive_path(source, &archive_name, stage_root, options);
    let provider_progress =
        download_progress.map(|observer| ForwardAddonDownloadProgressObserver {
            source,
            archive_name: &archive_name,
            inner: observer,
        });

    if let Some(cached_metadata) =
        cached_archive_metadata_if_local_file_matches(&archive_path, source, &archive_name)
    {
        let conditional_headers = conditional_request_headers_for_transport_validators(
            &cached_metadata.remote_validators,
        );
        if !conditional_headers.is_empty() {
            let response = download_to_path_with_headers(
                http_client,
                url,
                conditional_headers,
                &archive_path,
                cancellation,
                provider_progress
                    .as_ref()
                    .map(|observer| observer as &dyn HttpDownloadProgressObserver),
            )?;
            if response.is_not_modified() {
                return Ok(archive_path);
            }

            write_cached_archive_metadata(
                &archive_path,
                source,
                &archive_name,
                &remote_validators_for_http_headers(&response.headers),
                options,
            )?;
            return Ok(archive_path);
        }

        if should_reuse_cached_http_archive_without_transport_validators(&cached_metadata, options)
        {
            return Ok(archive_path);
        }
    }

    let response = download_to_path_with_headers(
        http_client,
        url,
        Vec::new(),
        &archive_path,
        cancellation,
        provider_progress
            .as_ref()
            .map(|observer| observer as &dyn HttpDownloadProgressObserver),
    )?;
    write_cached_archive_metadata(
        &archive_path,
        source,
        &archive_name,
        &remote_validators_for_http_headers(&response.headers),
        options,
    )?;
    Ok(archive_path)
}

fn should_reuse_cached_archive(
    source: &AddonSourceRef,
    archive_name: &str,
    remote_validators: &RemoteArchiveValidators,
    archive_path: &Path,
    options: &AddonProviderOptions,
) -> bool {
    options.download_cache_dir.is_some()
        && archive_path.is_file()
        && cached_archive_matches_metadata(archive_path, source, archive_name, remote_validators)
}

fn resolve_archive_path(
    source: &AddonSourceRef,
    archive_name: &str,
    stage_root: &Path,
    options: &AddonProviderOptions,
) -> PathBuf {
    let archive_name = normalize_archive_name(archive_name);
    match &options.download_cache_dir {
        Some(cache_dir) => cache_dir
            .join(source_cache_namespace(source))
            .join(short_hash(&source.display_name()))
            .join(archive_name),
        None => stage_root.join(archive_name),
    }
}

fn download_to_path_with_headers(
    http_client: &impl HttpClient,
    url: &str,
    headers: Vec<HttpHeader>,
    destination: &Path,
    cancellation: Option<&dyn CancellationToken>,
    observer: Option<&dyn HttpDownloadProgressObserver>,
) -> AppResult<HttpDownloadResponse> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary_destination = temporary_download_path(destination);
    if temporary_destination.exists() {
        fs::remove_file(&temporary_destination)?;
    }

    let never_cancel = NeverCancel;
    let cancellation = cancellation.unwrap_or(&never_cancel);
    let download_result = http_client.download_to_path(
        HttpDownloadRequest::new(url, temporary_destination.clone()).with_headers(headers),
        cancellation,
        observer,
    );
    let response = match download_result {
        Ok(response) => response,
        Err(error) => {
            let _ = fs::remove_file(&temporary_destination);
            return Err(error);
        }
    };

    if response.is_not_modified() {
        let _ = fs::remove_file(&temporary_destination);
        return Ok(response);
    }

    if destination.is_file() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary_destination, destination)?;
    Ok(response)
}

fn guess_archive_name_from_url(url: &str) -> Option<String> {
    if let Ok(parsed_url) = reqwest::Url::parse(url) {
        if let Some(file_name) = parsed_url
            .path_segments()
            .and_then(|segments| segments.filter(|segment| !segment.is_empty()).next_back())
            .filter(|segment| !segment.is_empty())
        {
            return Some(file_name.to_string());
        }
    }

    let stripped = url
        .split_once('#')
        .map_or(url, |(before_fragment, _)| before_fragment);
    let stripped = stripped
        .split_once('?')
        .map_or(stripped, |(before_query, _)| before_query);
    let file_name = Path::new(stripped).file_name()?.to_str()?;
    (!file_name.is_empty()).then(|| file_name.to_string())
}

fn normalize_archive_name(archive_name: &str) -> String {
    Path::new(archive_name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip")
        .to_string()
}

const TEMP_DOWNLOAD_SUFFIX: &str = ".hearthsync-part";
const CACHE_METADATA_SUFFIX: &str = ".hearthsync-cache.json";

fn temporary_download_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip");
    destination.with_file_name(format!("{file_name}{TEMP_DOWNLOAD_SUFFIX}"))
}

fn cached_archive_metadata_path(archive_path: &Path) -> PathBuf {
    let file_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip");
    archive_path.with_file_name(format!("{file_name}{CACHE_METADATA_SUFFIX}"))
}

fn write_cached_archive_metadata(
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
    remote_validators: &RemoteArchiveValidators,
    options: &AddonProviderOptions,
) -> AppResult<()> {
    if options.download_cache_dir.is_none() {
        return Ok(());
    }

    let metadata = CachedArchiveMetadata {
        source_display_name: source.display_name(),
        source_ref: Some(source.clone()),
        archive_name: normalize_archive_name(archive_name),
        file_size: fs::metadata(archive_path)?.len(),
        file_sha256: file_sha256(archive_path)?,
        fetched_at_unix_timestamp: Some(current_unix_timestamp_secs()?),
        remote_validators: remote_validators.clone(),
    };
    let metadata_bytes = serde_json::to_vec_pretty(&metadata)?;
    write_bytes_atomically(&cached_archive_metadata_path(archive_path), &metadata_bytes)
}

fn current_unix_timestamp_secs() -> AppResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| AppError::Validation("system clock is before unix epoch".to_string()))
}

fn cached_archive_metadata_if_local_file_matches(
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
) -> Option<CachedArchiveMetadata> {
    let metadata = load_cached_archive_metadata(archive_path)?;
    cached_archive_metadata_matches_local_file(&metadata, archive_path, source, archive_name)
        .then_some(metadata)
}

fn load_cached_archive_metadata(archive_path: &Path) -> Option<CachedArchiveMetadata> {
    let metadata_path = cached_archive_metadata_path(archive_path);
    let metadata_bytes = fs::read(&metadata_path).ok()?;
    serde_json::from_slice::<CachedArchiveMetadata>(&metadata_bytes).ok()
}

fn cached_archive_metadata_matches_local_file(
    metadata: &CachedArchiveMetadata,
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
) -> bool {
    if metadata.source_display_name != source.display_name() {
        return false;
    }
    if metadata.archive_name != normalize_archive_name(archive_name) {
        return false;
    }

    let Ok(file_metadata) = fs::metadata(archive_path) else {
        return false;
    };
    if metadata.file_size != file_metadata.len() {
        return false;
    }

    let Ok(file_sha256) = file_sha256(archive_path) else {
        return false;
    };
    metadata.file_sha256 == file_sha256
}

fn cached_archive_matches_metadata(
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
    remote_validators: &RemoteArchiveValidators,
) -> bool {
    let Some(metadata) =
        cached_archive_metadata_if_local_file_matches(archive_path, source, archive_name)
    else {
        return false;
    };

    if remote_validators.is_empty() {
        return true;
    }

    metadata.remote_validators == *remote_validators
}

fn should_reuse_cached_http_archive_without_transport_validators(
    metadata: &CachedArchiveMetadata,
    options: &AddonProviderOptions,
) -> bool {
    let Some(max_age_secs) = options.http_no_validator_cache_policy.max_age_secs() else {
        return false;
    };
    let Some(fetched_at_unix_timestamp) = metadata.fetched_at_unix_timestamp else {
        return false;
    };
    let Ok(now) = current_unix_timestamp_secs() else {
        return false;
    };
    if fetched_at_unix_timestamp > now {
        return false;
    }

    now - fetched_at_unix_timestamp <= max_age_secs
}

fn purge_download_cache_dir(cache_dir: Option<&Path>) -> AppResult<AddonDownloadCachePurgeResult> {
    let Some(cache_dir) = cache_dir else {
        return Ok(AddonDownloadCachePurgeResult::not_configured());
    };

    validate_cache_root(cache_dir)?;
    let mut stats = RemovedPathStats::default();
    if !cache_dir.exists() {
        return Ok(AddonDownloadCachePurgeResult::for_cache_dir(
            cache_dir.to_path_buf(),
            stats,
        ));
    }

    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        remove_path_recursively(&entry.path(), &mut stats)?;
    }

    Ok(AddonDownloadCachePurgeResult::for_cache_dir(
        cache_dir.to_path_buf(),
        stats,
    ))
}

fn repair_download_cache_dir(
    http_client: &impl HttpClient,
    cache_dir: Option<&Path>,
    options: &AddonProviderOptions,
) -> AppResult<AddonDownloadCacheRepairResult> {
    let Some(cache_dir) = cache_dir else {
        return Ok(AddonDownloadCacheRepairResult::not_configured());
    };

    validate_cache_root(cache_dir)?;
    let mut result = AddonDownloadCacheRepairResult::for_cache_dir(cache_dir.to_path_buf());
    if !cache_dir.exists() {
        return Ok(result);
    }

    let files = cache_file_paths(cache_dir)?;
    let mut stats = RemovedPathStats::default();

    for metadata_path in files.iter().filter(|path| is_cache_metadata_path(path)) {
        result.scanned_metadata_count += 1;
        repair_metadata_entry(http_client, metadata_path, options, &mut result, &mut stats)?;
    }

    for file_path in files {
        if is_cache_metadata_path(&file_path) || !file_path.is_file() {
            continue;
        }

        if is_temporary_download_path(&file_path) {
            if remove_path_if_exists(&file_path, &mut stats)? {
                result.partial_download_count += 1;
                result.repaired_entry_count += 1;
            }
            continue;
        }

        if cached_archive_metadata_path(&file_path).is_file() {
            continue;
        }

        if remove_path_if_exists(&file_path, &mut stats)? {
            result.orphan_archive_count += 1;
            result.repaired_entry_count += 1;
        }
    }

    remove_empty_cache_directories(cache_dir, &mut stats)?;
    result.removed_file_count = stats.removed_file_count;
    result.removed_directory_count = stats.removed_directory_count;
    result.reclaimed_bytes = stats.reclaimed_bytes;

    Ok(result)
}

fn repair_metadata_entry(
    http_client: &impl HttpClient,
    metadata_path: &Path,
    options: &AddonProviderOptions,
    result: &mut AddonDownloadCacheRepairResult,
    stats: &mut RemovedPathStats,
) -> AppResult<()> {
    let metadata_bytes = fs::read(metadata_path);
    let metadata = metadata_bytes
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CachedArchiveMetadata>(&bytes).ok());

    let Some(metadata) = metadata else {
        result.invalid_metadata_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        if let Some(archive_path) = archive_path_from_metadata_sidecar(metadata_path) {
            remove_path_if_exists(&archive_path, stats)?;
        }
        return Ok(());
    };

    let Some(archive_path) = archive_path_from_metadata_sidecar(metadata_path) else {
        result.invalid_metadata_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        return Ok(());
    };

    let archive_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_default();
    let metadata_valid = !metadata.source_display_name.trim().is_empty()
        && !metadata.archive_name.trim().is_empty()
        && metadata.archive_name == archive_name;

    if !metadata_valid {
        result.invalid_metadata_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        remove_path_if_exists(&archive_path, stats)?;
        return Ok(());
    }

    if !archive_path.is_file() {
        result.missing_archive_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        return Ok(());
    }

    let archive_matches = fs::metadata(&archive_path)
        .map(|file_metadata| file_metadata.len() == metadata.file_size)
        .unwrap_or(false)
        && file_sha256(&archive_path)
            .map(|sha256| sha256 == metadata.file_sha256)
            .unwrap_or(false);

    if !archive_matches {
        result.mismatched_archive_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        remove_path_if_exists(&archive_path, stats)?;
        return Ok(());
    }

    match repair_remote_cache_entry(http_client, &archive_path, &metadata, options) {
        Ok(CacheRemoteRepairStatus::Unchanged) => {
            result.remote_verified_entry_count += 1;
        }
        Ok(CacheRemoteRepairStatus::Refreshed) => {
            result.remote_refreshed_entry_count += 1;
            result.repaired_entry_count += 1;
        }
        Ok(CacheRemoteRepairStatus::Expired) => {
            result.expired_freshness_entry_count += 1;
            result.repaired_entry_count += 1;
            remove_path_if_exists(metadata_path, stats)?;
            remove_path_if_exists(&archive_path, stats)?;
        }
        Ok(CacheRemoteRepairStatus::Failed) | Err(_) => {
            result.remote_check_failed_count += 1;
        }
        Ok(CacheRemoteRepairStatus::Skipped) => {}
    }

    Ok(())
}

fn repair_remote_cache_entry(
    http_client: &impl HttpClient,
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
    options: &AddonProviderOptions,
) -> AppResult<CacheRemoteRepairStatus> {
    let Some(source_ref) = cached_source_ref_from_metadata(metadata) else {
        return Ok(CacheRemoteRepairStatus::Skipped);
    };

    match source_ref {
        AddonSourceRef::HttpArchive { ref url } => repair_http_archive_cache_entry(
            http_client,
            archive_path,
            metadata,
            &source_ref,
            url,
            options,
        ),
        AddonSourceRef::GitHubRelease {
            ref owner,
            ref repo,
            tag: Some(ref tag),
            asset_name: Some(ref asset_name),
        } => repair_github_archive_cache_entry(
            http_client,
            archive_path,
            metadata,
            &source_ref,
            owner,
            repo,
            tag,
            asset_name,
            options,
        ),
        AddonSourceRef::CurseForgeMod {
            mod_id,
            file_id: Some(file_id),
        } => repair_curseforge_archive_cache_entry(
            http_client,
            archive_path,
            metadata,
            &source_ref,
            mod_id,
            file_id,
            options,
        ),
        _ => Ok(CacheRemoteRepairStatus::Skipped),
    }
}

fn repair_http_archive_cache_entry(
    http_client: &impl HttpClient,
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
    source_ref: &AddonSourceRef,
    url: &str,
    options: &AddonProviderOptions,
) -> AppResult<CacheRemoteRepairStatus> {
    let conditional_headers =
        conditional_request_headers_for_transport_validators(&metadata.remote_validators);
    if !conditional_headers.is_empty() {
        let response = match download_to_path_with_headers(
            http_client,
            url,
            conditional_headers,
            archive_path,
            None,
            None,
        ) {
            Ok(response) => response,
            Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
        };
        if response.is_not_modified() {
            return Ok(CacheRemoteRepairStatus::Unchanged);
        }

        write_cached_archive_metadata(
            archive_path,
            source_ref,
            &metadata.archive_name,
            &remote_validators_for_http_headers(&response.headers),
            options,
        )?;
        return Ok(CacheRemoteRepairStatus::Refreshed);
    }

    if should_reuse_cached_http_archive_without_transport_validators(metadata, options) {
        return Ok(CacheRemoteRepairStatus::Skipped);
    }

    Ok(CacheRemoteRepairStatus::Expired)
}

fn repair_github_archive_cache_entry(
    http_client: &impl HttpClient,
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
    source_ref: &AddonSourceRef,
    owner: &str,
    repo: &str,
    tag: &str,
    asset_name: &str,
    options: &AddonProviderOptions,
) -> AppResult<CacheRemoteRepairStatus> {
    let release = match fetch_github_release_with_client(http_client, owner, repo, Some(tag)) {
        Ok(release) => release,
        Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
    };
    let asset = match select_github_release_asset(&release, Some(asset_name)) {
        Ok(asset) => asset,
        Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
    };
    let remote_validators = remote_validators_for_github_asset(asset);
    if remote_validators.is_empty() {
        return Ok(CacheRemoteRepairStatus::Skipped);
    }
    if remote_validators == metadata.remote_validators {
        return Ok(CacheRemoteRepairStatus::Unchanged);
    }

    refresh_cached_archive(
        http_client,
        archive_path,
        source_ref,
        &metadata.archive_name,
        &asset.browser_download_url,
        Vec::new(),
        &remote_validators,
        options,
    )?;
    Ok(CacheRemoteRepairStatus::Refreshed)
}

fn repair_curseforge_archive_cache_entry(
    http_client: &impl HttpClient,
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
    source_ref: &AddonSourceRef,
    mod_id: u32,
    file_id: u32,
    options: &AddonProviderOptions,
) -> AppResult<CacheRemoteRepairStatus> {
    let file =
        match resolve_curseforge_file_with_client(http_client, mod_id, Some(file_id), None, None) {
            Ok(file) => file,
            Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
        };
    let Some(download_url) = file.download_url.clone() else {
        return Ok(CacheRemoteRepairStatus::Failed);
    };
    let remote_validators = remote_validators_for_curseforge_file(&file);
    if remote_validators.is_empty() {
        return Ok(CacheRemoteRepairStatus::Skipped);
    }
    if remote_validators == metadata.remote_validators {
        return Ok(CacheRemoteRepairStatus::Unchanged);
    }

    refresh_cached_archive(
        http_client,
        archive_path,
        source_ref,
        &metadata.archive_name,
        &download_url,
        Vec::new(),
        &remote_validators,
        options,
    )?;
    Ok(CacheRemoteRepairStatus::Refreshed)
}

fn refresh_cached_archive(
    http_client: &impl HttpClient,
    archive_path: &Path,
    source_ref: &AddonSourceRef,
    archive_name: &str,
    download_url: &str,
    headers: Vec<HttpHeader>,
    remote_validators: &RemoteArchiveValidators,
    options: &AddonProviderOptions,
) -> AppResult<()> {
    download_to_path_with_headers(http_client, download_url, headers, archive_path, None, None)?;
    write_cached_archive_metadata(
        archive_path,
        source_ref,
        archive_name,
        remote_validators,
        options,
    )
}

fn cached_source_ref_from_metadata(metadata: &CachedArchiveMetadata) -> Option<AddonSourceRef> {
    metadata
        .source_ref
        .clone()
        .or_else(|| parse_cached_source_display_name(&metadata.source_display_name))
}

fn parse_cached_source_display_name(source_display_name: &str) -> Option<AddonSourceRef> {
    if let Ok(Some(source_ref)) = parse_curseforge_source(source_display_name) {
        return Some(source_ref);
    }
    if let Ok(Some(source_ref)) = parse_github_source(source_display_name) {
        return Some(source_ref);
    }
    if source_display_name.starts_with("https://") || source_display_name.starts_with("http://") {
        return Some(AddonSourceRef::HttpArchive {
            url: source_display_name.to_string(),
        });
    }

    None
}

fn validate_cache_root(cache_dir: &Path) -> AppResult<()> {
    if cache_dir.exists() && !cache_dir.is_dir() {
        return Err(AppError::Validation(format!(
            "configured addon download cache path is not a directory: {}",
            cache_dir.display()
        )));
    }

    Ok(())
}

fn cache_file_paths(cache_dir: &Path) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(cache_dir).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Io(std::io::Error::other(error)))?;
        if entry.file_type().is_file() {
            files.push(entry.into_path());
        }
    }

    Ok(files)
}

fn remove_empty_cache_directories(cache_dir: &Path, stats: &mut RemovedPathStats) -> AppResult<()> {
    let mut directories = WalkDir::new(cache_dir)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));

    for directory in directories {
        if fs::read_dir(&directory)?.next().is_none() {
            fs::remove_dir(&directory)?;
            stats.removed_directory_count += 1;
        }
    }

    Ok(())
}

fn remove_path_if_exists(path: &Path, stats: &mut RemovedPathStats) -> AppResult<bool> {
    if !path.exists() {
        return Ok(false);
    }

    remove_path_recursively(path, stats)?;
    Ok(true)
}

fn remove_path_recursively(path: &Path, stats: &mut RemovedPathStats) -> AppResult<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            remove_path_recursively(&entry.path(), stats)?;
        }
        fs::remove_dir(path)?;
        stats.removed_directory_count += 1;
        return Ok(());
    }

    fs::remove_file(path)?;
    stats.removed_file_count += 1;
    stats.reclaimed_bytes += metadata.len();
    Ok(())
}

fn is_cache_metadata_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(CACHE_METADATA_SUFFIX))
}

fn is_temporary_download_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(TEMP_DOWNLOAD_SUFFIX))
}

fn archive_path_from_metadata_sidecar(metadata_path: &Path) -> Option<PathBuf> {
    let file_name = metadata_path.file_name()?.to_str()?;
    let archive_name = file_name.strip_suffix(CACHE_METADATA_SUFFIX)?;
    Some(metadata_path.with_file_name(archive_name))
}

fn remote_validators_for_github_asset(asset: &GitHubReleaseAsset) -> RemoteArchiveValidators {
    let mut validators = RemoteArchiveValidators {
        content_length: asset.size,
        last_modified: asset.updated_at.clone(),
        etag: None,
        sha256: None,
        sha1: None,
        md5: None,
    };

    if let Some(digest) = asset.digest.as_deref() {
        if let Some(value) = digest.strip_prefix("sha256:") {
            validators.sha256 = Some(value.to_ascii_lowercase());
        }
    }

    validators
}

const CURSEFORGE_HASH_ALGO_SHA1: u8 = 1;
const CURSEFORGE_HASH_ALGO_MD5: u8 = 2;

fn remote_validators_for_curseforge_file(file: &CurseForgeFile) -> RemoteArchiveValidators {
    let mut validators = RemoteArchiveValidators {
        content_length: file.file_length,
        last_modified: (!file.file_date.is_empty()).then(|| file.file_date.clone()),
        etag: None,
        sha256: None,
        sha1: None,
        md5: None,
    };

    for hash in &file.hashes {
        if hash.value.is_empty() {
            continue;
        }
        match hash.algo {
            CURSEFORGE_HASH_ALGO_SHA1 if validators.sha1.is_none() => {
                validators.sha1 = Some(hash.value.to_ascii_lowercase());
            }
            CURSEFORGE_HASH_ALGO_MD5 if validators.md5.is_none() => {
                validators.md5 = Some(hash.value.to_ascii_lowercase());
            }
            _ => {}
        }
    }

    validators
}

fn remote_validators_for_http_headers(headers: &[HttpHeader]) -> RemoteArchiveValidators {
    RemoteArchiveValidators {
        content_length: header_value_case_insensitive(headers, "content-length")
            .and_then(|value| value.parse::<u64>().ok()),
        last_modified: header_value_case_insensitive(headers, "last-modified"),
        etag: header_value_case_insensitive(headers, "etag"),
        sha256: None,
        sha1: None,
        md5: None,
    }
}

fn header_value_case_insensitive(headers: &[HttpHeader], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn conditional_request_headers_for_transport_validators(
    validators: &RemoteArchiveValidators,
) -> Vec<HttpHeader> {
    let mut headers = Vec::new();
    if let Some(etag) = &validators.etag {
        headers.push(HttpHeader {
            name: "If-None-Match".to_string(),
            value: etag.clone(),
        });
    }
    if let Some(last_modified) = &validators.last_modified {
        headers.push(HttpHeader {
            name: "If-Modified-Since".to_string(),
            value: last_modified.clone(),
        });
    }
    headers
}

const CURSEFORGE_REQUIRED_DEPENDENCY_RELATION_TYPE: u8 = 3;

fn required_dependency_sources_for_curseforge_file(
    source_mod_id: u32,
    dependencies: &[CurseForgeFileDependency],
) -> Vec<AddonSourceRef> {
    let mut dependency_mod_ids = dependencies
        .iter()
        .filter(|dependency| {
            dependency.relation_type == CURSEFORGE_REQUIRED_DEPENDENCY_RELATION_TYPE
        })
        .map(|dependency| dependency.mod_id)
        .filter(|mod_id| *mod_id != 0 && *mod_id != source_mod_id)
        .collect::<Vec<_>>();
    dependency_mod_ids.sort_unstable();
    dependency_mod_ids.dedup();

    dependency_mod_ids
        .into_iter()
        .map(|mod_id| AddonSourceRef::CurseForgeMod {
            mod_id,
            file_id: None,
        })
        .collect()
}

fn file_sha256(path: &Path) -> AppResult<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn source_cache_namespace(source: &AddonSourceRef) -> &'static str {
    match source {
        AddonSourceRef::LocalArchive { .. } => "local",
        AddonSourceRef::HttpArchive { .. } => "http",
        AddonSourceRef::CurseForgeMod { .. } => "curseforge",
        AddonSourceRef::GitHubRelease { .. } => "github",
    }
}

fn source_kind_label(source: &AddonSourceRef) -> &'static str {
    match source {
        AddonSourceRef::LocalArchive { .. } => "local_archive",
        AddonSourceRef::HttpArchive { .. } => "http_archive",
        AddonSourceRef::CurseForgeMod { .. } => "curseforge_mod",
        AddonSourceRef::GitHubRelease { .. } => "github_release",
    }
}

fn short_hash(value: &str) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn github_allows_prerelease(policy: AddonSourceResolutionPolicy) -> bool {
    match policy.allow_prerelease {
        Some(value) => value,
        None => matches!(
            policy.release_channel,
            Some(AddonReleaseChannel::Beta | AddonReleaseChannel::Alpha)
        ),
    }
}

fn curseforge_release_type_limit(
    policy: AddonSourceResolutionPolicy,
) -> Option<CurseForgeFileReleaseType> {
    if matches!(policy.allow_prerelease, Some(false)) {
        return Some(CurseForgeFileReleaseType::Stable);
    }

    match policy.release_channel {
        Some(AddonReleaseChannel::Stable) => Some(CurseForgeFileReleaseType::Stable),
        Some(AddonReleaseChannel::Beta) => Some(CurseForgeFileReleaseType::Beta),
        Some(AddonReleaseChannel::Alpha) => Some(CurseForgeFileReleaseType::Alpha),
        None if matches!(policy.allow_prerelease, Some(true)) => {
            Some(CurseForgeFileReleaseType::Alpha)
        }
        None => None,
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

struct ForwardAddonDownloadProgressObserver<'a> {
    source: &'a AddonSourceRef,
    archive_name: &'a str,
    inner: &'a dyn AddonDownloadProgressObserver,
}

impl HttpDownloadProgressObserver for ForwardAddonDownloadProgressObserver<'_> {
    fn on_progress(&self, progress: HttpDownloadProgress) {
        self.inner.on_download_progress(
            self.source,
            self.archive_name,
            progress.bytes_current,
            progress.bytes_total,
            progress.bytes_per_second,
        );
    }
}

#[cfg(windows)]
fn normalize_canonical_archive_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(stripped) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", stripped));
    }
    if let Some(stripped) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }
    path
}

#[cfg(not(windows))]
fn normalize_canonical_archive_path(path: PathBuf) -> PathBuf {
    path
}
