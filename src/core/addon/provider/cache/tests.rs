use std::cell::RefCell;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use super::super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpHeader, HttpRequest, HttpResponse,
};
use super::super::test_support::{
    NoopHttpClient, load_cached_archive_metadata_fixture, not_modified_download_response,
    successful_download_response, write_cached_archive_metadata_fixture,
};
use super::super::validation::RemoteArchiveValidators;
use super::super::{AddonProviderOptions, AddonSourceRef};
use super::*;
use crate::core::error::AppResult;
use crate::core::task::CancellationToken;

#[test]
fn guess_archive_name_from_url_ignores_query_string_and_fragment() {
    assert_eq!(
        guess_archive_name_from_url("https://example.com/downloads/addon.zip?token=abc123#section")
            .as_deref(),
        Some("addon.zip")
    );
}

#[test]
fn download_to_path_rejects_directory_destination_before_http_call() {
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
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let destination = temp.path().join("cache").join("addon.zip");
    std::fs::create_dir_all(&destination).expect("destination directory");
    let http_client = FakeHttpClient::default();

    let error = download_to_path_with_headers(
        &http_client,
        "https://example.com/addon.zip",
        Vec::new(),
        &destination,
        None,
        None,
    )
    .expect_err("directory destination should fail before download");

    assert_eq!(*http_client.downloads.borrow(), 0);
    assert!(error.to_string().contains("not a replaceable file"));
    assert!(
        !destination
            .with_file_name("addon.zip.hearthsync-part")
            .exists()
    );
}

#[test]
fn download_to_path_removes_temporary_file_when_final_destination_becomes_unreplaceable() {
    #[derive(Default)]
    struct FakeHttpClient {
        temporary_destination: RefCell<Option<PathBuf>>,
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
            *self.temporary_destination.borrow_mut() = Some(request.destination.clone());
            std::fs::write(&request.destination, b"archive").expect("temporary archive");
            let final_file_name = request
                .destination
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_suffix(".hearthsync-part"))
                .expect("temporary suffix");
            std::fs::create_dir(request.destination.with_file_name(final_file_name))
                .expect("race-created destination directory");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let destination = temp.path().join("cache").join("addon.zip");
    let http_client = FakeHttpClient::default();

    let error = download_to_path_with_headers(
        &http_client,
        "https://example.com/addon.zip",
        Vec::new(),
        &destination,
        None,
        None,
    )
    .expect_err("post-download directory destination should fail");

    let temporary_destination = http_client
        .temporary_destination
        .borrow()
        .clone()
        .expect("temporary destination");
    assert!(error.to_string().contains("not a replaceable file"));
    assert!(destination.is_dir());
    assert!(!temporary_destination.exists());
}

#[test]
fn purge_download_cache_removes_cached_files() {
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

    let result = purge_download_cache_dir(Some(&cache_dir)).expect("purge cache");

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
fn repair_download_cache_removes_invalid_entries_and_orphans() {
    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let options = cache_options(&cache_dir);

    let valid_source = AddonSourceRef::HttpArchive {
        url: "https://example.com/valid.zip".to_string(),
    };
    let valid_archive = write_cache_entry(temp.path(), &options, &valid_source, "valid.zip", b"ok");

    let invalid_source = AddonSourceRef::HttpArchive {
        url: "https://example.com/broken.zip".to_string(),
    };
    let invalid_archive = write_cache_entry(
        temp.path(),
        &options,
        &invalid_source,
        "broken.zip",
        b"broken",
    );
    std::fs::write(cached_archive_metadata_path(&invalid_archive), b"{not-json")
        .expect("broken metadata");

    let missing_source = AddonSourceRef::HttpArchive {
        url: "https://example.com/missing.zip".to_string(),
    };
    let missing_archive = write_cache_entry(
        temp.path(),
        &options,
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
        temp.path(),
        &options,
        &mismatch_source,
        "mismatch.zip",
        b"match",
    );
    std::fs::write(&mismatch_archive, b"mutated").expect("mutate archive");

    let orphan_archive = resolve_archive_path(
        &AddonSourceRef::HttpArchive {
            url: "https://example.com/orphan.zip".to_string(),
        },
        "orphan.zip",
        temp.path(),
        &options,
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

    let result = repair_download_cache_dir(&NoopHttpClient, Some(&cache_dir), &options)
        .expect("repair cache");

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
    assert!(cached_archive_metadata_path(&valid_archive).is_file());
    assert!(!invalid_archive.exists());
    assert!(!cached_archive_metadata_path(&invalid_archive).exists());
    assert!(!cached_archive_metadata_path(&missing_archive).exists());
    assert!(!mismatch_archive.exists());
    assert!(!cached_archive_metadata_path(&mismatch_archive).exists());
    assert!(!orphan_archive.exists());
    assert!(!partial_path.exists());
}

#[test]
fn repair_download_cache_prunes_expired_http_archives_without_validators() {
    let temp = tempdir().expect("temp dir");
    let cache_dir = temp.path().join("cache");
    let options = AddonProviderOptions {
        download_cache_dir: Some(cache_dir.clone()),
        http_no_validator_cache_policy: HttpNoValidatorCachePolicy::ReuseWithinWindow {
            max_age_secs: 60,
        },
        ..AddonProviderOptions::default()
    };
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };
    let archive_path = write_cache_entry(temp.path(), &options, &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.fetched_at_unix_timestamp = Some(0);
    write_cached_archive_metadata_fixture(&archive_path, &metadata);

    let result = repair_download_cache_dir(&NoopHttpClient, Some(&cache_dir), &options)
        .expect("repair cache");

    assert_eq!(result.scanned_metadata_count, 1);
    assert_eq!(result.repaired_entry_count, 1);
    assert_eq!(result.expired_freshness_entry_count, 1);
    assert_eq!(result.remote_verified_entry_count, 0);
    assert_eq!(result.remote_refreshed_entry_count, 0);
    assert!(!archive_path.exists());
    assert!(!cached_archive_metadata_path(&archive_path).exists());
}

#[test]
fn repair_download_cache_verifies_http_archives_with_conditional_get() {
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
    let options = cache_options(&cache_dir);
    let http_client = FakeHttpClient::default();
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };
    let archive_path = write_cache_entry(temp.path(), &options, &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.remote_validators = RemoteArchiveValidators {
        content_length: Some(7),
        last_modified: Some("Wed, 23 Apr 2026 10:00:00 GMT".to_string()),
        etag: Some("\"addon-v1\"".to_string()),
        sha256: None,
        sha1: None,
        md5: None,
    };
    write_cached_archive_metadata_fixture(&archive_path, &metadata);

    let result =
        repair_download_cache_dir(&http_client, Some(&cache_dir), &options).expect("repair cache");

    assert_eq!(result.scanned_metadata_count, 1);
    assert_eq!(result.repaired_entry_count, 0);
    assert_eq!(result.remote_verified_entry_count, 1);
    assert_eq!(result.remote_refreshed_entry_count, 0);
    assert_eq!(result.remote_check_failed_count, 0);
    assert_eq!(http_client.downloads.borrow().len(), 1);
    assert_eq!(
        http_client.downloads.borrow()[0].headers,
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
fn repair_download_cache_refreshes_http_archives_when_remote_changed() {
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
    let cache_dir = temp.path().join("cache");
    let options = cache_options(&cache_dir);
    let http_client = FakeHttpClient::default();
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };
    let archive_path = write_cache_entry(temp.path(), &options, &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.remote_validators = RemoteArchiveValidators {
        content_length: Some(7),
        last_modified: Some("Wed, 23 Apr 2026 10:00:00 GMT".to_string()),
        etag: Some("\"addon-v1\"".to_string()),
        sha256: None,
        sha1: None,
        md5: None,
    };
    write_cached_archive_metadata_fixture(&archive_path, &metadata);

    let result =
        repair_download_cache_dir(&http_client, Some(&cache_dir), &options).expect("repair cache");
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
fn repair_download_cache_refreshes_github_archives_when_remote_validators_change() {
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
    let cache_dir = temp.path().join("cache");
    let options = cache_options(&cache_dir);
    let http_client = FakeHttpClient::default();
    let source = AddonSourceRef::GitHubRelease {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        tag: Some("v1.0.0".to_string()),
        asset_name: Some("addon.zip".to_string()),
    };
    let archive_path = write_cache_entry(temp.path(), &options, &source, "addon.zip", b"archive");
    let mut metadata = load_cached_archive_metadata_fixture(&archive_path);
    metadata.remote_validators = RemoteArchiveValidators {
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

    let result =
        repair_download_cache_dir(&http_client, Some(&cache_dir), &options).expect("repair cache");
    let repaired_metadata = load_cached_archive_metadata_fixture(&archive_path);

    assert_eq!(result.scanned_metadata_count, 1);
    assert_eq!(result.repaired_entry_count, 1);
    assert_eq!(result.remote_verified_entry_count, 0);
    assert_eq!(result.remote_refreshed_entry_count, 1);
    assert_eq!(result.remote_check_failed_count, 0);
    assert_eq!(http_client.requests.borrow().len(), 1);
    assert_eq!(http_client.downloads.borrow().len(), 1);
    assert_eq!(
        std::fs::read_to_string(&archive_path).expect("refreshed archive"),
        "archive-v2!"
    );
    assert_eq!(
        repaired_metadata.remote_validators.sha256,
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string())
    );
}

fn cache_options(cache_dir: &Path) -> AddonProviderOptions {
    AddonProviderOptions {
        download_cache_dir: Some(cache_dir.to_path_buf()),
        ..AddonProviderOptions::default()
    }
}

fn write_cache_entry(
    stage_root: &Path,
    options: &AddonProviderOptions,
    source: &AddonSourceRef,
    archive_name: &str,
    contents: &[u8],
) -> PathBuf {
    let archive_path = resolve_archive_path(source, archive_name, stage_root, options);
    std::fs::create_dir_all(archive_path.parent().expect("archive parent directory"))
        .expect("archive parent directory");
    std::fs::write(&archive_path, contents).expect("archive contents");
    write_cached_archive_metadata(
        &archive_path,
        source,
        archive_name,
        &RemoteArchiveValidators::default(),
        options,
    )
    .expect("cache metadata");
    archive_path
}
