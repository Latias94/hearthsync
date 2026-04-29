use std::path::{Path, PathBuf};

use super::cache::{
    cached_archive_metadata_if_local_file_matches, download_to_path_with_headers,
    guess_archive_name_from_url, resolve_archive_path, should_reuse_cached_archive,
    should_reuse_cached_http_archive_without_transport_validators, write_cached_archive_metadata,
};
use super::curseforge::resolve_curseforge_file_with_client;
use super::github::{
    fetch_github_release_with_client, fetch_github_releases_with_client, select_github_release,
    select_github_release_asset,
};
use super::http::{HttpClient, HttpDownloadProgress, HttpDownloadProgressObserver, HttpHeader};
use super::parse::{parse_curseforge_source, parse_github_source};
use super::source::{canonicalize_local_archive_path, validate_absolute_local_archive_source_path};
use super::source_adapter::{curseforge_release_type_limit, github_allows_prerelease};
use super::validation::{
    RemoteArchiveValidators, conditional_request_headers_for_transport_validators,
    remote_validators_for_curseforge_file, remote_validators_for_github_asset,
    remote_validators_for_http_headers,
};
use super::{
    AddonDownloadProgressObserver, AddonProviderContext, AddonProviderOptions, AddonSourceRef,
    MaterializedAddonSource,
};
use crate::core::boundary_validation::is_http_url;
use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;

#[derive(Debug, Clone)]
struct ResolvedDownloadArtifact {
    cache_source_ref: AddonSourceRef,
    download_url: String,
    archive_name: String,
    headers: Vec<HttpHeader>,
    remote_validators: RemoteArchiveValidators,
}

pub(super) fn materialize_source_input_impl(
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

    if is_http_url(source) {
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

pub(super) fn materialize_source_ref_impl(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    stage_root: &Path,
    context: AddonProviderContext<'_>,
    options: &AddonProviderOptions,
) -> AppResult<MaterializedAddonSource> {
    match source {
        AddonSourceRef::LocalArchive { path } => {
            validate_absolute_local_archive_source_path(path)?;
            Ok(MaterializedAddonSource {
                source_ref: source.clone(),
                archive_path: path.clone(),
            })
        }
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
