mod curseforge;
mod github;
mod http;
mod parse;
#[cfg(test)]
mod tests;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use self::curseforge::{resolve_curseforge_file_with_client, search_curseforge_mods_with_client};
use self::github::{fetch_github_release_with_client, select_github_release_asset};
use self::http::{HttpClient, HttpDownloadRequest, HttpHeader, HttpRequest, ReqwestHttpClient};
use self::parse::{parse_curseforge_source, parse_github_source};
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

#[derive(Clone, Copy, Default)]
pub struct AddonProviderContext<'a> {
    pub target_flavor: Option<WowFlavor>,
    pub cancellation: Option<&'a dyn CancellationToken>,
}

impl fmt::Debug for AddonProviderContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddonProviderContext")
            .field("target_flavor", &self.target_flavor)
            .field("has_cancellation", &self.cancellation.is_some())
            .finish()
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddonProviderOptions {
    pub download_cache_dir: Option<PathBuf>,
    pub retry_policy: AddonProviderRetryPolicy,
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
    ) -> AppResult<()> {
        retry_http(self.max_attempts, || {
            self.inner.download_to_path(request.clone(), cancellation)
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
            let file_name = guess_archive_name_from_url(url).unwrap_or("downloaded-addon.zip");
            let archive_path = materialize_downloaded_archive(
                http_client,
                source,
                url,
                file_name,
                Vec::new(),
                stage_root,
                context.cancellation,
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
            )?;
            let download_url = file.download_url.clone().ok_or_else(|| {
                AppError::Validation(format!(
                    "CurseForge file `{}` does not provide a download URL",
                    file.id
                ))
            })?;
            let archive_path = materialize_downloaded_archive(
                http_client,
                source,
                &download_url,
                &file.file_name,
                Vec::new(),
                stage_root,
                context.cancellation,
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
            let release =
                fetch_github_release_with_client(http_client, owner, repo, tag.as_deref())?;
            let asset = select_github_release_asset(&release, asset_name.as_deref())?;
            let archive_path = materialize_downloaded_archive(
                http_client,
                source,
                &asset.browser_download_url,
                &asset.name,
                Vec::new(),
                stage_root,
                context.cancellation,
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

fn materialize_downloaded_archive(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    url: &str,
    archive_name: &str,
    headers: Vec<HttpHeader>,
    stage_root: &Path,
    cancellation: Option<&dyn CancellationToken>,
    options: &AddonProviderOptions,
) -> AppResult<PathBuf> {
    let archive_path = resolve_archive_path(source, archive_name, stage_root, options);
    if options.download_cache_dir.is_some() && archive_path.is_file() {
        return Ok(archive_path);
    }

    download_to_path_with_headers(http_client, url, headers, &archive_path, cancellation)?;
    Ok(archive_path)
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
) -> AppResult<()> {
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
    );
    if let Err(error) = download_result {
        let _ = fs::remove_file(&temporary_destination);
        return Err(error);
    }

    if destination.is_file() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary_destination, destination)?;
    Ok(())
}

fn guess_archive_name_from_url(url: &str) -> Option<&str> {
    let file_name = Path::new(url).file_name()?.to_str()?;
    if file_name.is_empty() {
        None
    } else {
        Some(file_name)
    }
}

fn normalize_archive_name(archive_name: &str) -> String {
    Path::new(archive_name)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip")
        .to_string()
}

fn temporary_download_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip");
    destination.with_file_name(format!("{file_name}.hearthsync-part"))
}

fn source_cache_namespace(source: &AddonSourceRef) -> &'static str {
    match source {
        AddonSourceRef::LocalArchive { .. } => "local",
        AddonSourceRef::HttpArchive { .. } => "http",
        AddonSourceRef::CurseForgeMod { .. } => "curseforge",
        AddonSourceRef::GitHubRelease { .. } => "github",
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
