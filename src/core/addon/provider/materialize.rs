use std::path::{Path, PathBuf};

use super::cache::{
    cached_archive_metadata_if_local_file_matches, download_to_path_with_headers,
    guess_archive_name_from_url, resolve_archive_path, should_reuse_cached_archive,
    should_reuse_cached_http_archive_without_transport_validators, write_cached_archive_metadata,
};
use super::curseforge::{
    remote_validators_for_curseforge_file, resolve_curseforge_file_with_client,
};
use super::github::{
    fetch_github_release_with_client, fetch_github_releases_with_client,
    remote_validators_for_github_asset, select_github_release, select_github_release_asset,
};
use super::http::{HttpClient, HttpDownloadProgress, HttpDownloadProgressObserver, HttpHeader};
use super::parse::{parse_curseforge_source, parse_github_source};
use super::source::{canonicalize_local_archive_path, validate_absolute_local_archive_source_path};
use super::source_adapter::{curseforge_release_type_limit, github_allows_prerelease};
use super::validation::{
    RemoteArchiveValidators, conditional_request_headers_for_transport_validators,
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::super::AddonSourceResolutionPolicy;
    use super::super::http::{
        HttpDownloadRequest, HttpDownloadResponse, HttpRequest, HttpResponse,
    };
    use super::super::test_support::NoopHttpClient;
    use super::*;

    #[test]
    fn materialize_source_ref_rejects_relative_local_archive_source_refs() {
        let temp = tempdir().expect("temp dir");
        let source = AddonSourceRef::LocalArchive {
            path: PathBuf::from("addons/WeakAuras.zip"),
        };

        let error = materialize_source_ref_impl(
            &NoopHttpClient,
            &source,
            temp.path(),
            AddonProviderContext::default(),
            &AddonProviderOptions::default(),
        )
        .expect_err("relative persisted local source should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(error.to_string().contains("must be absolute"));
    }

    #[test]
    fn materialize_http_archive_forwards_download_progress_to_observer() {
        #[derive(Default)]
        struct FakeHttpClient;

        impl HttpClient for FakeHttpClient {
            fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
                panic!("get should not be called in this test")
            }

            fn download_to_path(
                &self,
                request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                let observer = observer.expect("download observer");
                observer.on_progress(HttpDownloadProgress {
                    bytes_current: 0,
                    bytes_total: Some(1024),
                    bytes_per_second: None,
                });
                observer.on_progress(HttpDownloadProgress {
                    bytes_current: 1024,
                    bytes_total: Some(1024),
                    bytes_per_second: Some(512),
                });
                std::fs::write(&request.destination, b"archive").expect("archive file");
                Ok(HttpDownloadResponse {
                    status_code: 200,
                    headers: Vec::new(),
                })
            }
        }

        type ProgressEvent = (String, String, u64, Option<u64>, Option<u64>);

        #[derive(Default)]
        struct FakeObserver {
            seen: RefCell<Vec<ProgressEvent>>,
        }

        impl AddonDownloadProgressObserver for FakeObserver {
            fn on_download_progress(
                &self,
                source: &AddonSourceRef,
                archive_name: &str,
                bytes_current: u64,
                bytes_total: Option<u64>,
                bytes_per_second: Option<u64>,
            ) {
                self.seen.borrow_mut().push((
                    source.display_name(),
                    archive_name.to_string(),
                    bytes_current,
                    bytes_total,
                    bytes_per_second,
                ));
            }
        }

        let temp = tempdir().expect("temp dir");
        let source = AddonSourceRef::HttpArchive {
            url: "https://example.com/addon.zip".to_string(),
        };
        let observer = FakeObserver::default();

        let materialized = materialize_source_ref_impl(
            &FakeHttpClient,
            &source,
            temp.path(),
            AddonProviderContext::new(None, None).with_download_progress(Some(&observer)),
            &AddonProviderOptions::default(),
        )
        .expect("materialize source");

        assert!(materialized.archive_path.exists());
        assert_eq!(
            observer.seen.borrow().as_slice(),
            &[
                (
                    "https://example.com/addon.zip".to_string(),
                    "addon.zip".to_string(),
                    0,
                    Some(1024),
                    None,
                ),
                (
                    "https://example.com/addon.zip".to_string(),
                    "addon.zip".to_string(),
                    1024,
                    Some(1024),
                    Some(512),
                ),
            ]
        );
    }

    #[test]
    fn materialize_github_release_selects_prerelease_when_policy_allows_it() {
        #[derive(Default)]
        struct FakeHttpClient {
            requests: RefCell<Vec<HttpRequest>>,
            downloads: RefCell<Vec<HttpDownloadRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                self.requests.borrow_mut().push(request.clone());
                if request.url.ends_with("/releases") {
                    return Ok(HttpResponse {
                        status_code: 200,
                        body: r#"[{"tag_name":"v2.0.0-beta.1","prerelease":true,"assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v2.0.0-beta.1/addon.zip"}]},{"tag_name":"v1.9.9","prerelease":false,"assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.9.9/addon.zip"}]}]"#.to_string(),
                    });
                }
                Err(AppError::Validation(format!(
                    "unexpected request url: {}",
                    request.url
                )))
            }

            fn download_to_path(
                &self,
                request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                self.downloads.borrow_mut().push(request.clone());
                std::fs::write(&request.destination, request.url.as_bytes()).expect("archive file");
                Ok(HttpDownloadResponse {
                    status_code: 200,
                    headers: Vec::new(),
                })
            }
        }

        let temp = tempdir().expect("temp dir");
        let http_client = FakeHttpClient::default();
        let source = AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: Some("addon.zip".to_string()),
        };

        let materialized = materialize_source_ref_impl(
            &http_client,
            &source,
            temp.path(),
            AddonProviderContext::default().with_resolution_policy(AddonSourceResolutionPolicy {
                release_channel: None,
                allow_prerelease: Some(true),
            }),
            &AddonProviderOptions::default(),
        )
        .expect("materialize prerelease github source");

        assert_eq!(
            http_client.requests.borrow()[0].url,
            "https://api.github.com/repos/owner/repo/releases"
        );
        assert_eq!(http_client.downloads.borrow().len(), 1);
        assert_eq!(
            std::fs::read_to_string(&materialized.archive_path).expect("downloaded archive"),
            "https://example.com/releases/v2.0.0-beta.1/addon.zip"
        );
    }
}
