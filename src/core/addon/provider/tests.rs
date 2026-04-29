use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpHeader, HttpRequest, HttpResponse,
};
use super::test_support::curseforge_api_key_guard;
use super::{
    AddonProvider, AddonProviderContext, AddonSourceRef, AddonSourceResolutionPolicy,
    DefaultAddonProvider, HttpNoValidatorCachePolicy,
};
use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;

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
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let http_client = FakeHttpClient::default();
    let provider = DefaultAddonProvider::with_http_client(http_client)
        .with_download_cache_dir(Some(temp.path().join("cache")));

    let materialized = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let mut metadata = load_cached_archive_metadata_fixture(&first.archive_path);
    metadata.fetched_at_unix_timestamp = Some(0);
    write_cached_archive_metadata_fixture(&first.archive_path, &metadata);

    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let mut metadata = load_cached_archive_metadata_fixture(&first.archive_path);
    metadata.fetched_at_unix_timestamp = None;
    write_cached_archive_metadata_fixture(&first.archive_path, &metadata);

    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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
fn default_addon_provider_purge_download_cache_removes_cached_files() {
    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let nested_dir = cache_dir.join("http").join("deadbeef");
    std::fs::create_dir_all(&nested_dir).expect("cache dir");
    std::fs::write(nested_dir.join("addon.zip"), b"archive").expect("archive");
    std::fs::write(
        nested_dir.join("addon.zip.hearthsync-cache.json"),
        br#"{"kind":"fixture"}"#,
    )
    .expect("metadata");
    std::fs::write(nested_dir.join("addon.zip.hearthsync-part"), b"partial").expect("part");

    let provider = DefaultAddonProvider::default().with_download_cache_dir(Some(cache_dir.clone()));

    let result = provider.purge_download_cache().expect("purge cache");

    assert_eq!(result.cache_dir, Some(cache_dir.clone()));
    assert_eq!(result.removed_file_count, 3);
    assert_eq!(result.removed_directory_count, 2);
    assert_eq!(result.reclaimed_bytes, 32);
    assert!(
        std::fs::read_dir(&cache_dir)
            .expect("cache dir entries")
            .next()
            .is_none()
    );
}

#[test]
fn default_addon_provider_repair_download_cache_removes_invalid_entries_and_orphans() {
    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let provider = DefaultAddonProvider::default().with_download_cache_dir(Some(cache_dir.clone()));

    let valid_source = AddonSourceRef::HttpArchive {
        url: "https://example.com/valid.zip".to_string(),
    };
    let valid_archive =
        write_cache_entry(&provider, temp.path(), &valid_source, "valid.zip", b"ok");

    let invalid_source = AddonSourceRef::HttpArchive {
        url: "https://example.com/broken.zip".to_string(),
    };
    let invalid_archive = write_cache_entry(
        &provider,
        temp.path(),
        &invalid_source,
        "broken.zip",
        b"broken",
    );
    std::fs::write(
        super::cached_archive_metadata_path(&invalid_archive),
        b"{not-json",
    )
    .expect("broken metadata");

    let missing_source = AddonSourceRef::HttpArchive {
        url: "https://example.com/missing.zip".to_string(),
    };
    let missing_archive = write_cache_entry(
        &provider,
        temp.path(),
        &missing_source,
        "missing.zip",
        b"missing",
    );
    std::fs::remove_file(&missing_archive).expect("remove archive");

    let mismatch_source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: Some("v1.0.0".to_string()),
        asset_name: Some("mismatch.zip".to_string()),
    };
    let mismatch_archive = write_cache_entry(
        &provider,
        temp.path(),
        &mismatch_source,
        "mismatch.zip",
        b"match",
    );
    std::fs::write(&mismatch_archive, b"mutated").expect("mutate archive");

    let orphan_archive = super::resolve_archive_path(
        &AddonSourceRef::HttpArchive {
            url: "https://example.com/orphan.zip".to_string(),
        },
        "orphan.zip",
        temp.path(),
        provider.options(),
    );
    std::fs::create_dir_all(
        orphan_archive
            .parent()
            .expect("orphan archive parent directory"),
    )
    .expect("orphan dir");
    std::fs::write(&orphan_archive, b"orphan").expect("orphan archive");

    let partial_path = orphan_archive.with_file_name("pending.zip.hearthsync-part");
    std::fs::write(&partial_path, b"partial").expect("partial download");

    let result = provider.repair_download_cache().expect("repair cache");

    assert_eq!(result.cache_dir, Some(cache_dir));
    assert_eq!(result.scanned_metadata_count, 4);
    assert_eq!(result.repaired_entry_count, 5);
    assert_eq!(result.invalid_metadata_count, 1);
    assert_eq!(result.missing_archive_count, 1);
    assert_eq!(result.mismatched_archive_count, 1);
    assert_eq!(result.orphan_archive_count, 1);
    assert_eq!(result.partial_download_count, 1);
    assert_eq!(result.remote_verified_entry_count, 0);
    assert_eq!(result.remote_refreshed_entry_count, 0);
    assert_eq!(result.remote_check_failed_count, 0);
    assert_eq!(result.expired_freshness_entry_count, 0);
    assert_eq!(result.removed_file_count, 7);
    assert_eq!(result.removed_directory_count, 5);
    assert!(result.reclaimed_bytes >= 37);

    assert!(valid_archive.is_file());
    assert!(super::cached_archive_metadata_path(&valid_archive).is_file());
    assert!(!invalid_archive.exists());
    assert!(!super::cached_archive_metadata_path(&invalid_archive).exists());
    assert!(!super::cached_archive_metadata_path(&missing_archive).exists());
    assert!(!mismatch_archive.exists());
    assert!(!super::cached_archive_metadata_path(&mismatch_archive).exists());
    assert!(!orphan_archive.exists());
    assert!(!partial_path.exists());
}

#[test]
fn default_addon_provider_repair_download_cache_verifies_http_archives_with_conditional_get() {
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
    let archive_path = write_cache_entry(&provider, temp.path(), &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.remote_validators = super::RemoteArchiveValidators {
        content_length: Some(7),
        last_modified: Some("Wed, 23 Apr 2026 10:00:00 GMT".to_string()),
        etag: Some("\"addon-v1\"".to_string()),
        sha256: None,
        sha1: None,
        md5: None,
    };
    write_cached_archive_metadata_fixture(&archive_path, &metadata);

    let result = provider.repair_download_cache().expect("repair cache");

    assert_eq!(result.scanned_metadata_count, 1);
    assert_eq!(result.repaired_entry_count, 0);
    assert_eq!(result.remote_verified_entry_count, 1);
    assert_eq!(result.remote_refreshed_entry_count, 0);
    assert_eq!(result.remote_check_failed_count, 0);
    assert_eq!(provider.http_client().downloads.borrow().len(), 1);
    assert_eq!(
        provider.http_client().downloads.borrow()[0].headers,
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
        std::fs::read_to_string(&archive_path).expect("cached archive"),
        "archive"
    );
}

#[test]
fn default_addon_provider_repair_download_cache_refreshes_http_archives_when_remote_changed() {
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
            std::fs::write(&request.destination, b"archive-v2").expect("archive file");
            Ok(successful_download_response(vec![
                HttpHeader {
                    name: "ETag".to_string(),
                    value: "\"addon-v2\"".to_string(),
                },
                HttpHeader {
                    name: "Last-Modified".to_string(),
                    value: "Thu, 24 Apr 2026 10:00:00 GMT".to_string(),
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
    let archive_path = write_cache_entry(&provider, temp.path(), &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.remote_validators = super::RemoteArchiveValidators {
        content_length: Some(7),
        last_modified: Some("Wed, 23 Apr 2026 10:00:00 GMT".to_string()),
        etag: Some("\"addon-v1\"".to_string()),
        sha256: None,
        sha1: None,
        md5: None,
    };
    write_cached_archive_metadata_fixture(&archive_path, &metadata);

    let result = provider.repair_download_cache().expect("repair cache");
    let repaired_metadata = load_cached_archive_metadata_fixture(&archive_path);

    assert_eq!(result.scanned_metadata_count, 1);
    assert_eq!(result.repaired_entry_count, 1);
    assert_eq!(result.remote_verified_entry_count, 0);
    assert_eq!(result.remote_refreshed_entry_count, 1);
    assert_eq!(result.remote_check_failed_count, 0);
    assert_eq!(
        std::fs::read_to_string(&archive_path).expect("refreshed archive"),
        "archive-v2"
    );
    assert_eq!(
        repaired_metadata.remote_validators.etag,
        Some("\"addon-v2\"".to_string())
    );
}

#[test]
fn default_addon_provider_repair_download_cache_refreshes_github_archives_when_remote_validators_change()
 {
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
                body: r#"{"tag_name":"v1.0.0","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/addon.zip","size":12,"digest":"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","updated_at":"2026-04-24T10:00:00Z"}]}"#.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            std::fs::write(&request.destination, b"archive-v2!").expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: Some("v1.0.0".to_string()),
        asset_name: Some("addon.zip".to_string()),
    };
    let archive_path = write_cache_entry(&provider, temp.path(), &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.remote_validators = super::RemoteArchiveValidators {
        content_length: Some(7),
        last_modified: Some("2026-04-23T10:00:00Z".to_string()),
        etag: None,
        sha256: Some(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        ),
        sha1: None,
        md5: None,
    };
    write_cached_archive_metadata_fixture(&archive_path, &metadata);

    let result = provider.repair_download_cache().expect("repair cache");
    let repaired_metadata = load_cached_archive_metadata_fixture(&archive_path);

    assert_eq!(result.scanned_metadata_count, 1);
    assert_eq!(result.repaired_entry_count, 1);
    assert_eq!(result.remote_verified_entry_count, 0);
    assert_eq!(result.remote_refreshed_entry_count, 1);
    assert_eq!(result.remote_check_failed_count, 0);
    assert_eq!(provider.http_client().requests.borrow().len(), 1);
    assert_eq!(provider.http_client().downloads.borrow().len(), 1);
    assert_eq!(
        std::fs::read_to_string(&archive_path).expect("refreshed archive"),
        "archive-v2!"
    );
    assert_eq!(
        repaired_metadata.remote_validators.sha256,
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string())
    );
}

#[test]
fn default_addon_provider_repair_download_cache_prunes_expired_http_archives_without_validators() {
    #[derive(Default)]
    struct FakeHttpClient;

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
            panic!("download should not be called in this test")
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient)
        .with_download_cache_dir(Some(temp.path().join("cache")))
        .with_http_no_validator_cache_policy(HttpNoValidatorCachePolicy::ReuseWithinWindow {
            max_age_secs: 60,
        });
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };
    let archive_path = write_cache_entry(&provider, temp.path(), &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.fetched_at_unix_timestamp = Some(0);
    write_cached_archive_metadata_fixture(&archive_path, &metadata);

    let result = provider.repair_download_cache().expect("repair cache");

    assert_eq!(result.scanned_metadata_count, 1);
    assert_eq!(result.repaired_entry_count, 1);
    assert_eq!(result.expired_freshness_entry_count, 1);
    assert_eq!(result.remote_verified_entry_count, 0);
    assert_eq!(result.remote_refreshed_entry_count, 0);
    assert!(!archive_path.exists());
    assert!(!super::cached_archive_metadata_path(&archive_path).exists());
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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

    assert_eq!(first.archive_path, second.archive_path);
    assert!(first.archive_path.starts_with(&cache_dir));
    assert!(super::cached_archive_metadata_path(&first.archive_path).is_file());
    assert_eq!(provider.http_client().requests.borrow().len(), 2);
    assert_eq!(provider.http_client().downloads.borrow().len(), 1);
}

#[test]
fn default_addon_provider_selects_github_prerelease_when_policy_allows_it() {
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
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default());
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: None,
        asset_name: Some("addon.zip".to_string()),
    };

    let materialized = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: temp.path(),
            context: AddonProviderContext::default().with_resolution_policy(
                AddonSourceResolutionPolicy {
                    release_channel: None,
                    allow_prerelease: Some(true),
                },
            ),
        })
        .expect("materialize prerelease github source");

    assert_eq!(
        provider.http_client().requests.borrow()[0].url,
        "https://api.github.com/repos/owner/repo/releases"
    );
    assert_eq!(provider.http_client().downloads.borrow().len(), 1);
    assert_eq!(
        std::fs::read_to_string(&materialized.archive_path).expect("downloaded archive"),
        "https://example.com/releases/v2.0.0-beta.1/addon.zip"
    );
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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    std::fs::remove_file(super::cached_archive_metadata_path(&first.archive_path))
        .expect("remove cache sidecar");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    std::fs::write(&first.archive_path, b"corrupted-cache").expect("corrupt cache file");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

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

    let first = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-a"),
            context: AddonProviderContext::default(),
        })
        .expect("first materialize");
    let second = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: &temp.path().join("stage-b"),
            context: AddonProviderContext::default(),
        })
        .expect("second materialize");

    assert_ne!(first.archive_path, second.archive_path);
    assert!(first.archive_path.starts_with(&cache_dir));
    assert!(second.archive_path.starts_with(&cache_dir));
    assert_eq!(provider.http_client().release_calls.get(), 2);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
}

fn write_cache_entry<H>(
    provider: &DefaultAddonProvider<H>,
    stage_root: &std::path::Path,
    source: &AddonSourceRef,
    archive_name: &str,
    contents: &[u8],
) -> PathBuf {
    let archive_path =
        super::resolve_archive_path(source, archive_name, stage_root, provider.options());
    std::fs::create_dir_all(archive_path.parent().expect("archive parent directory"))
        .expect("archive parent directory");
    std::fs::write(&archive_path, contents).expect("archive contents");
    super::write_cached_archive_metadata(
        &archive_path,
        source,
        archive_name,
        &super::RemoteArchiveValidators::default(),
        provider.options(),
    )
    .expect("cache metadata");
    archive_path
}

fn load_cached_archive_metadata_fixture(
    archive_path: &std::path::Path,
) -> super::CachedArchiveMetadata {
    let metadata_path = super::cached_archive_metadata_path(archive_path);
    let metadata_bytes = std::fs::read(metadata_path).expect("cache metadata bytes");
    serde_json::from_slice(&metadata_bytes).expect("cache metadata")
}

fn write_cached_archive_metadata_fixture(
    archive_path: &std::path::Path,
    metadata: &super::CachedArchiveMetadata,
) {
    let metadata_path = super::cached_archive_metadata_path(archive_path);
    let metadata_bytes = serde_json::to_vec_pretty(metadata).expect("cache metadata json");
    std::fs::write(metadata_path, metadata_bytes).expect("cache metadata write");
}

fn successful_download_response(headers: Vec<HttpHeader>) -> HttpDownloadResponse {
    HttpDownloadResponse {
        status_code: 200,
        headers,
    }
}

fn not_modified_download_response(headers: Vec<HttpHeader>) -> HttpDownloadResponse {
    HttpDownloadResponse {
        status_code: 304,
        headers,
    }
}
