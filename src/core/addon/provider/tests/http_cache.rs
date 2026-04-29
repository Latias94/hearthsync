use std::cell::RefCell;

use tempfile::tempdir;

use super::super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpHeader, HttpRequest, HttpResponse,
};
use super::super::test_support::{
    load_cached_archive_metadata_fixture, not_modified_download_response,
    successful_download_response, write_cached_archive_metadata_fixture,
};
use super::super::{AddonSourceRef, DefaultAddonProvider, HttpNoValidatorCachePolicy};
use super::{materialize_source, materialize_source_twice};
use crate::core::error::AppResult;
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
