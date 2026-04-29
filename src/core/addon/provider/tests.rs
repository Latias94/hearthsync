use std::cell::{Cell, RefCell};
use std::path::Path;
use tempfile::tempdir;

use super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpHeader, HttpRequest, HttpResponse,
};
use super::test_support::{
    cached_metadata_path, curseforge_api_key_guard, load_cached_archive_metadata_fixture,
    not_modified_download_response, successful_download_response,
    write_cached_archive_metadata_fixture,
};
use super::{
    AddonProvider, AddonProviderContext, AddonSourceRef, DefaultAddonProvider,
    HttpNoValidatorCachePolicy,
};
use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;

#[test]
fn default_addon_provider_refreshes_download_cache_for_http_archives_when_policy_requires_it() {
    #[derive(Default)]
    struct FakeHttpClient {
        downloads: RefCell<Vec<HttpDownloadRequest>>,
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
            self.downloads.borrow_mut().push(request.clone());
            std::fs::write(&request.destination, b"archive").expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(cache_dir.clone()))
        .with_http_no_validator_cache_policy(HttpNoValidatorCachePolicy::AlwaysRefresh);
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert!(first.archive_path.starts_with(&cache_dir));
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
}

#[test]
fn default_addon_provider_reuses_download_cache_for_http_archives_within_no_validator_freshness_window()
 {
    #[derive(Default)]
    struct FakeHttpClient {
        downloads: RefCell<Vec<HttpDownloadRequest>>,
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
            self.downloads.borrow_mut().push(request.clone());
            std::fs::write(&request.destination, b"archive").expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(cache_dir.clone()));
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert!(first.archive_path.starts_with(&cache_dir));
    assert_eq!(provider.http_client().downloads.borrow().len(), 1);
}

#[test]
fn default_addon_provider_redownloads_cached_http_archive_when_no_validator_cache_entry_is_stale() {
    #[derive(Default)]
    struct FakeHttpClient {
        downloads: RefCell<Vec<HttpDownloadRequest>>,
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
            self.downloads.borrow_mut().push(request.clone());
            let call = self.downloads.borrow().len();
            std::fs::write(&request.destination, format!("archive-{call}")).expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(cache_dir.clone()))
        .with_http_no_validator_cache_policy(HttpNoValidatorCachePolicy::ReuseWithinWindow {
            max_age_secs: 60,
        });
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };

    let first = materialize_source(&provider, &source, temp.path(), "stage-a");
    let mut metadata = load_cached_archive_metadata_fixture(&first.archive_path);
    metadata.fetched_at_unix_timestamp = Some(0);
    write_cached_archive_metadata_fixture(&first.archive_path, &metadata);

    let second = materialize_source(&provider, &source, temp.path(), "stage-b");

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        std::fs::read_to_string(&second.archive_path).expect("refreshed archive"),
        "archive-2"
    );
}

#[test]
fn default_addon_provider_redownloads_cached_http_archive_when_legacy_no_validator_sidecar_lacks_fetch_time()
 {
    #[derive(Default)]
    struct FakeHttpClient {
        downloads: RefCell<Vec<HttpDownloadRequest>>,
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
            self.downloads.borrow_mut().push(request.clone());
            let call = self.downloads.borrow().len();
            std::fs::write(&request.destination, format!("archive-{call}")).expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };

    let first = materialize_source(&provider, &source, temp.path(), "stage-a");
    let mut metadata = load_cached_archive_metadata_fixture(&first.archive_path);
    metadata.fetched_at_unix_timestamp = None;
    write_cached_archive_metadata_fixture(&first.archive_path, &metadata);

    let second = materialize_source(&provider, &source, temp.path(), "stage-b");

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        std::fs::read_to_string(&second.archive_path).expect("refreshed archive"),
        "archive-2"
    );
}

#[test]
fn default_addon_provider_reuses_download_cache_for_http_archives_when_conditional_get_returns_not_modified()
 {
    #[derive(Default)]
    struct FakeHttpClient {
        downloads: RefCell<Vec<HttpDownloadRequest>>,
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
            self.downloads.borrow_mut().push(request.clone());
            if self.downloads.borrow().len() == 1 {
                std::fs::write(&request.destination, b"archive").expect("archive file");
                return Ok(successful_download_response(vec![
                    HttpHeader {
                        name: "ETag".to_string(),
                        value: "\"addon-v1\"".to_string(),
                    },
                    HttpHeader {
                        name: "Last-Modified".to_string(),
                        value: "Wed, 23 Apr 2026 10:00:00 GMT".to_string(),
                    },
                    HttpHeader {
                        name: "Content-Length".to_string(),
                        value: "7".to_string(),
                    },
                ]));
            }

            Ok(not_modified_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(cache_dir.clone()));
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert!(first.archive_path.starts_with(&cache_dir));
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        provider.http_client().downloads.borrow()[1].headers,
        vec![
            HttpHeader {
                name: "If-None-Match".to_string(),
                value: "\"addon-v1\"".to_string(),
            },
            HttpHeader {
                name: "If-Modified-Since".to_string(),
                value: "Wed, 23 Apr 2026 10:00:00 GMT".to_string(),
            },
        ]
    );
}

#[test]
fn default_addon_provider_redownloads_cached_http_archive_when_conditional_get_returns_fresh_payload()
 {
    #[derive(Default)]
    struct FakeHttpClient {
        downloads: RefCell<Vec<HttpDownloadRequest>>,
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
            self.downloads.borrow_mut().push(request.clone());
            let call = self.downloads.borrow().len();
            let payload = format!("archive-download-{call}");
            std::fs::write(&request.destination, payload).expect("archive file");
            let etag = if call == 1 {
                "\"addon-v1\""
            } else {
                "\"addon-v2\""
            };
            Ok(successful_download_response(vec![
                HttpHeader {
                    name: "ETag".to_string(),
                    value: etag.to_string(),
                },
                HttpHeader {
                    name: "Last-Modified".to_string(),
                    value: "Wed, 23 Apr 2026 10:00:00 GMT".to_string(),
                },
            ]))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        provider.http_client().downloads.borrow()[1].headers,
        vec![
            HttpHeader {
                name: "If-None-Match".to_string(),
                value: "\"addon-v1\"".to_string(),
            },
            HttpHeader {
                name: "If-Modified-Since".to_string(),
                value: "Wed, 23 Apr 2026 10:00:00 GMT".to_string(),
            },
        ]
    );
    assert_eq!(
        std::fs::read_to_string(&second.archive_path).expect("refreshed archive"),
        "archive-download-2"
    );
}

#[test]
fn default_addon_provider_reuses_download_cache_for_resolved_latest_github_release() {
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
                body: r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip"}]}"#.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            std::fs::write(&request.destination, b"archive-v1.2.3").expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(cache_dir.clone()));
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: None,
        asset_name: None,
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert!(first.archive_path.starts_with(&cache_dir));
    assert!(cached_metadata_path(&first.archive_path).is_file());
    assert_eq!(provider.http_client().requests.borrow().len(), 2);
    assert_eq!(provider.http_client().downloads.borrow().len(), 1);
}

#[test]
fn default_addon_provider_redownloads_cached_release_when_cache_metadata_is_missing() {
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
                body: r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip"}]}"#.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            std::fs::write(&request.destination, b"archive-v1.2.3").expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: None,
        asset_name: None,
    };

    let first = materialize_source(&provider, &source, temp.path(), "stage-a");
    std::fs::remove_file(cached_metadata_path(&first.archive_path)).expect("remove cache sidecar");
    let second = materialize_source(&provider, &source, temp.path(), "stage-b");

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
}

#[test]
fn default_addon_provider_redownloads_cached_release_when_cached_archive_is_modified() {
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
                body: r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip"}]}"#.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            let payload = format!("archive-download-{}", self.downloads.borrow().len());
            std::fs::write(&request.destination, payload).expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: None,
        asset_name: None,
    };

    let first = materialize_source(&provider, &source, temp.path(), "stage-a");
    std::fs::write(&first.archive_path, b"corrupted-cache").expect("corrupt cache file");
    let second = materialize_source(&provider, &source, temp.path(), "stage-b");

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        std::fs::read_to_string(&second.archive_path).expect("refreshed archive"),
        "archive-download-2"
    );
}

#[test]
fn default_addon_provider_redownloads_cached_release_when_remote_asset_validator_changes() {
    struct FakeHttpClient {
        release_calls: Cell<usize>,
        downloads: RefCell<Vec<HttpDownloadRequest>>,
    }

    impl Default for FakeHttpClient {
        fn default() -> Self {
            Self {
                release_calls: Cell::new(0),
                downloads: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for FakeHttpClient {
        fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
            let next_call = self.release_calls.get() + 1;
            self.release_calls.set(next_call);
            let body = match next_call {
                1 => {
                    r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip","size":17,"digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111","updated_at":"2026-04-20T10:00:00Z"}]}"#
                }
                _ => {
                    r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip","size":21,"digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222","updated_at":"2026-04-21T10:00:00Z"}]}"#
                }
            };
            Ok(HttpResponse {
                status_code: 200,
                body: body.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            let payload = format!("archive-download-{}", self.downloads.borrow().len());
            std::fs::write(&request.destination, payload).expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: Some("v1.2.3".to_string()),
        asset_name: Some("addon.zip".to_string()),
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().release_calls.get(), 2);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        std::fs::read_to_string(&second.archive_path).expect("refreshed archive"),
        "archive-download-2"
    );
}

#[test]
fn default_addon_provider_redownloads_cached_curseforge_file_when_remote_validator_changes() {
    let _guard = curseforge_api_key_guard("test-api-key");

    struct FakeHttpClient {
        file_calls: Cell<usize>,
        downloads: RefCell<Vec<HttpDownloadRequest>>,
    }

    impl Default for FakeHttpClient {
        fn default() -> Self {
            Self {
                file_calls: Cell::new(0),
                downloads: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for FakeHttpClient {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            if !request.url.ends_with("/mods/42/files/777") {
                return Err(AppError::Validation(format!(
                    "unexpected request url: {}",
                    request.url
                )));
            }
            let next_call = self.file_calls.get() + 1;
            self.file_calls.set(next_call);
            let body = match next_call {
                1 => {
                    r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-20T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"fileLength":17,"hashes":[{"value":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","algo":1},{"value":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","algo":2}]}}"#
                }
                _ => {
                    r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"fileLength":21,"hashes":[{"value":"cccccccccccccccccccccccccccccccccccccccc","algo":1},{"value":"dddddddddddddddddddddddddddddddd","algo":2}]}}"#
                }
            };
            Ok(HttpResponse {
                status_code: 200,
                body: body.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            let payload = format!("archive-download-{}", self.downloads.borrow().len());
            std::fs::write(&request.destination, payload).expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::CurseForgeMod {
        mod_id: 42,
        file_id: Some(777),
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().file_calls.get(), 2);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        std::fs::read_to_string(&second.archive_path).expect("refreshed archive"),
        "archive-download-2"
    );
}

#[test]
fn default_addon_provider_refreshes_latest_github_release_when_resolved_tag_changes() {
    struct FakeHttpClient {
        release_calls: Cell<usize>,
        downloads: RefCell<Vec<HttpDownloadRequest>>,
    }

    impl Default for FakeHttpClient {
        fn default() -> Self {
            Self {
                release_calls: Cell::new(0),
                downloads: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for FakeHttpClient {
        fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
            let next_call = self.release_calls.get() + 1;
            self.release_calls.set(next_call);
            let body = match next_call {
                1 => {
                    r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip"}]}"#
                }
                _ => {
                    r#"{"tag_name":"v1.2.4","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.4/addon.zip"}]}"#
                }
            };
            Ok(HttpResponse {
                status_code: 200,
                body: body.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            std::fs::write(&request.destination, request.url.as_bytes()).expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(cache_dir.clone()));
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: None,
        asset_name: None,
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_ne!(first.archive_path, second.archive_path);
    assert!(first.archive_path.starts_with(&cache_dir));
    assert!(second.archive_path.starts_with(&cache_dir));
    assert_eq!(provider.http_client().release_calls.get(), 2);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
}

fn materialize_source_twice<H>(
    provider: &DefaultAddonProvider<H>,
    source: &AddonSourceRef,
    stage_root: &Path,
) -> (
    super::MaterializedAddonSource,
    super::MaterializedAddonSource,
)
where
    H: HttpClient,
{
    (
        materialize_source(provider, source, stage_root, "stage-a"),
        materialize_source(provider, source, stage_root, "stage-b"),
    )
}

fn materialize_source<H>(
    provider: &DefaultAddonProvider<H>,
    source: &AddonSourceRef,
    stage_root: &Path,
    stage_name: &str,
) -> super::MaterializedAddonSource
where
    H: HttpClient,
{
    let stage_root = stage_root.join(stage_name);
    provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source,
            stage_root: &stage_root,
            context: AddonProviderContext::default(),
        })
        .unwrap_or_else(|error| panic!("materialize {stage_name}: {error}"))
}
