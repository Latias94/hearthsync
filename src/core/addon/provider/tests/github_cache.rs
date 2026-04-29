use std::cell::{Cell, RefCell};

use tempfile::tempdir;

use super::super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpRequest, HttpResponse,
};
use super::super::test_support::{cached_metadata_path, successful_download_response};
use super::super::{AddonSourceRef, DefaultAddonProvider};
use super::{materialize_source, materialize_source_twice};
use crate::core::error::AppResult;
use crate::core::task::CancellationToken;

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
