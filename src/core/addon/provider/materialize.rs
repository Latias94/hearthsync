use std::path::{Path, PathBuf};

#[cfg(test)]
use super::AddonProviderContext;
use super::cache::{
    cached_archive_metadata_if_local_file_matches, download_to_path_with_headers,
    resolve_archive_path, should_reuse_cached_archive,
    should_reuse_cached_http_archive_without_transport_validators, write_cached_archive_metadata,
};
use super::http::{HttpClient, HttpDownloadProgress, HttpDownloadProgressObserver, HttpHeader};
#[cfg(test)]
use super::registry::AddonProviderRegistry;
use super::registry::{http_archive_artifact_name, validate_persisted_local_source};
use super::validation::{
    RemoteArchiveValidators, conditional_request_headers_for_transport_validators,
    remote_validators_for_http_headers,
};
use super::{
    AddonDownloadProgressObserver, AddonProviderOptions, AddonSourceRef, MaterializedAddonSource,
};
use crate::core::boundary_validation::validate_http_url;
#[cfg(test)]
use crate::core::error::AppError;
use crate::core::error::AppResult;
use crate::core::task::CancellationToken;

#[derive(Debug, Clone)]
pub(super) struct ResolvedDownloadArtifact {
    pub(super) cache_source_ref: AddonSourceRef,
    pub(super) download_url: String,
    pub(super) archive_name: String,
    pub(super) headers: Vec<HttpHeader>,
    pub(super) remote_validators: RemoteArchiveValidators,
}

#[cfg(test)]
pub(super) fn materialize_source_ref_impl(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    stage_root: &Path,
    context: AddonProviderContext<'_>,
    options: &AddonProviderOptions,
) -> AppResult<MaterializedAddonSource> {
    AddonProviderRegistry::new().materialize_source_ref(
        http_client,
        source,
        stage_root,
        context,
        options,
    )
}

pub(super) fn materialize_local_archive_source(
    source: &AddonSourceRef,
    path: &Path,
) -> AppResult<MaterializedAddonSource> {
    validate_persisted_local_source(path)?;
    Ok(MaterializedAddonSource {
        source_ref: source.clone(),
        archive_path: path.to_path_buf(),
    })
}

pub(super) fn materialize_http_archive_source(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    url: &str,
    stage_root: &Path,
    cancellation: Option<&dyn CancellationToken>,
    download_progress: Option<&dyn AddonDownloadProgressObserver>,
    options: &AddonProviderOptions,
) -> AppResult<MaterializedAddonSource> {
    validate_http_url(url, "HTTP archive source URL")?;
    let archive_path = materialize_http_archive(
        http_client,
        source,
        url,
        stage_root,
        cancellation,
        download_progress,
        options,
    )?;
    Ok(MaterializedAddonSource {
        source_ref: source.clone(),
        archive_path,
    })
}

pub(super) fn materialize_downloaded_archive(
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
    let archive_name = http_archive_artifact_name(url);
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
    fn materialize_source_ref_rejects_invalid_http_archive_url_before_download() {
        #[derive(Default)]
        struct FakeHttpClient {
            downloads: RefCell<usize>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
                panic!("get should not be called in this test")
            }

            fn download_to_path(
                &self,
                _request: HttpDownloadRequest,
                _cancellation: &dyn CancellationToken,
                _observer: Option<&dyn HttpDownloadProgressObserver>,
            ) -> AppResult<HttpDownloadResponse> {
                *self.downloads.borrow_mut() += 1;
                Ok(HttpDownloadResponse {
                    status_code: 200,
                    headers: Vec::new(),
                })
            }
        }

        let temp = tempdir().expect("temp dir");
        let http_client = FakeHttpClient::default();
        let source = AddonSourceRef::HttpArchive {
            url: "https://example.com/addon.zip ".to_string(),
        };

        let error = materialize_source_ref_impl(
            &http_client,
            &source,
            temp.path(),
            AddonProviderContext::default(),
            &AddonProviderOptions::default(),
        )
        .expect_err("invalid HTTP archive URL should fail before download");

        assert_eq!(*http_client.downloads.borrow(), 0);
        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error
                .to_string()
                .contains("HTTP archive source URL must not have surrounding whitespace")
        );
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

    #[test]
    fn materialize_wago_addon_downloads_selected_release_link() {
        #[derive(Default)]
        struct FakeHttpClient {
            requests: RefCell<Vec<HttpRequest>>,
            downloads: RefCell<Vec<HttpDownloadRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                self.requests.borrow_mut().push(request.clone());
                if request.url == "https://addons.wago.io/addons/qv63A7Gb/versions" {
                    return Ok(HttpResponse {
                        status_code: 200,
                        body: wago_release_page_html("vdx1042w"),
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
        let source = AddonSourceRef::WagoAddon {
            project_id: "qv63A7Gb".to_string(),
            release_id: None,
        };

        let materialized = materialize_source_ref_impl(
            &http_client,
            &source,
            temp.path(),
            AddonProviderContext::new(Some(crate::core::install::WowFlavor::Retail), None),
            &AddonProviderOptions::default(),
        )
        .expect("materialize wago source");

        let requests = http_client.requests.borrow();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].query,
            vec![
                ("stability".to_string(), "stable".to_string()),
                ("page".to_string(), "1".to_string())
            ]
        );
        assert_eq!(http_client.downloads.borrow().len(), 1);
        assert_eq!(
            std::fs::read_to_string(&materialized.archive_path).expect("downloaded archive"),
            "https://addons.wago.io/download/vdx1042w?x=1&y=2"
        );
        assert_eq!(
            materialized
                .archive_path
                .file_name()
                .and_then(|name| name.to_str()),
            Some("wago-qv63A7Gb-vdx1042w.zip")
        );
    }

    fn wago_release_page_html(release_id: &str) -> String {
        let json = format!(
            r#"{{"component":"Addon/Releases","props":{{"releases":{{"current_page":1,"last_page":1,"data":[{{"id":"{release_id}","size":1024,"label":"Details","stability":"stable","created_at":"2026-05-01T00:00:00Z","is_processed":true,"supported_retail_patches":["12.0.5"],"download_link":"https://addons.wago.io/download/{release_id}?x=1&y=2"}}]}}}}}}"#
        );
        format!(
            r#"<html><body><div id="app" data-page="{}"></div></body></html>"#,
            json.replace('&', "&amp;").replace('"', "&quot;")
        )
    }
}
