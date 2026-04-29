use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::curseforge::{
    CurseForgeFile, CurseForgeFileReleaseType, CurseForgeGameVersionType,
    CurseForgeSortableGameVersion, select_curseforge_version_type, select_latest_curseforge_file,
    validate_curseforge_file,
};
use super::github::{
    GitHubRelease, GitHubReleaseAsset, fetch_github_release_with_client,
    fetch_github_releases_with_client, select_github_release, select_github_release_asset,
};
use super::http::{
    HttpClient, HttpDownloadProgress, HttpDownloadProgressObserver, HttpDownloadRequest,
    HttpDownloadResponse, HttpHeader, HttpRequest, HttpResponse, ReqwestHttpClient,
};
use super::parse::{parse_curseforge_source, parse_github_source};
use super::{
    AddonDependencyResolutionCapability, AddonDependencyResolutionStrategy,
    AddonDownloadProgressObserver, AddonProvider, AddonProviderContext, AddonSourceRef,
    AddonSourceResolutionPolicy, DefaultAddonProvider, HttpNoValidatorCachePolicy,
    ResolveAddonDependenciesRequest,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;
use crate::core::task::CancellationToken;

#[derive(Debug, Deserialize, Serialize)]
struct AddonSourceFixture {
    source: AddonSourceRef,
}

#[test]
fn addon_source_ref_uses_canonical_provider_kind_names() {
    let github = AddonSourceFixture {
        source: AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: None,
        },
    };
    let curseforge = AddonSourceFixture {
        source: AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: None,
        },
    };

    assert!(
        toml::to_string(&github)
            .expect("github source toml")
            .contains("kind = \"github_release\"")
    );
    assert!(
        toml::to_string(&curseforge)
            .expect("curseforge source toml")
            .contains("kind = \"curseforge_mod\"")
    );
}

#[test]
fn addon_source_ref_accepts_legacy_provider_kind_names() {
    let github: AddonSourceFixture = toml::from_str(
        r#"
source = { kind = "git_hub_release", owner = "owner", repo = "repo" }
"#,
    )
    .expect("legacy github source");
    let curseforge: AddonSourceFixture = toml::from_str(
        r#"
source = { kind = "curse_forge_mod", mod_id = 12345 }
"#,
    )
    .expect("legacy curseforge source");

    assert_eq!(
        github.source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: None,
        }
    );
    assert_eq!(
        curseforge.source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: None,
        }
    );
}

#[test]
fn default_addon_provider_rejects_relative_local_archive_source_refs() {
    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::default();

    let error = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &AddonSourceRef::LocalArchive {
                path: PathBuf::from("addons/WeakAuras.zip"),
            },
            stage_root: temp.path(),
            context: AddonProviderContext::default(),
        })
        .expect_err("relative persisted local source should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("must be absolute"));
}

#[test]
fn parse_curseforge_source_with_explicit_file() {
    let source = parse_curseforge_source("curseforge:12345@67890")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: Some(67890),
        }
    );
}

#[test]
fn parse_curseforge_source_without_file() {
    let source = parse_curseforge_source("curseforge:12345")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 12345,
            file_id: None,
        }
    );
}

#[test]
fn parse_curseforge_source_rejects_zero_ids() {
    for source in ["curseforge:0", "curseforge:12345@0"] {
        let error = parse_curseforge_source(source).expect_err("zero ids should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error
                .to_string()
                .contains("CurseForge source must look like")
        );
    }
}

#[test]
fn parse_github_source_with_tag_and_asset() {
    let source = parse_github_source("github:owner/repo@v1.2.3#addon.zip")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: Some("v1.2.3".to_string()),
            asset_name: Some("addon.zip".to_string()),
        }
    );
}

#[test]
fn parse_github_source_without_tag() {
    let source = parse_github_source("github:owner/repo")
        .expect("parse")
        .expect("source ref");

    assert_eq!(
        source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: None,
        }
    );
}

#[test]
fn fetch_github_release_with_client_uses_http_port() {
    #[derive(Default)]
    struct FakeHttpClient {
        requests: RefCell<Vec<HttpRequest>>,
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
            _request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            panic!("download_to_path should not be called in this test")
        }
    }

    let client = FakeHttpClient::default();
    let release = fetch_github_release_with_client(&client, "owner", "repo", Some("v1.2.3"))
        .expect("release");

    assert_eq!(release.assets.len(), 1);
    let requests = client.requests.borrow();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url,
        "https://api.github.com/repos/owner/repo/releases/tags/v1.2.3"
    );
    assert!(
        requests[0]
            .headers
            .iter()
            .any(|header| header.name == "Accept")
    );
    assert!(
        requests[0]
            .headers
            .iter()
            .any(|header| header.name == "User-Agent")
    );
    assert!(
        requests[0]
            .headers
            .iter()
            .any(|header| header.name == "X-GitHub-Api-Version")
    );
}

#[test]
fn fetch_github_releases_with_client_uses_release_list_endpoint() {
    #[derive(Default)]
    struct FakeHttpClient {
        requests: RefCell<Vec<HttpRequest>>,
    }

    impl HttpClient for FakeHttpClient {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            self.requests.borrow_mut().push(request);
            Ok(HttpResponse {
                status_code: 200,
                body: r#"[{"tag_name":"v1.2.3","prerelease":true,"assets":[{"name":"addon.zip","browser_download_url":"https://example.com/addon.zip"}]}]"#.to_string(),
            })
        }

        fn download_to_path(
            &self,
            _request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            panic!("download_to_path should not be called in this test")
        }
    }

    let client = FakeHttpClient::default();
    let releases = fetch_github_releases_with_client(&client, "owner", "repo").expect("releases");

    assert_eq!(releases.len(), 1);
    let requests = client.requests.borrow();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url,
        "https://api.github.com/repos/owner/repo/releases"
    );
}

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
fn default_addon_provider_resolves_required_curseforge_dependencies() {
    #[derive(Default)]
    struct FakeHttpClient {
        requests: RefCell<Vec<HttpRequest>>,
    }

    impl HttpClient for FakeHttpClient {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            self.requests.borrow_mut().push(request.clone());
            match request.url.as_str() {
                "https://api.curseforge.com/v1/mods/42/files/777" => Ok(HttpResponse {
                    status_code: 200,
                    body: r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"releaseType":1,"dependencies":[{"modId":99,"relationType":3},{"modId":100,"relationType":2},{"modId":42,"relationType":3}]}}"#.to_string(),
                }),
                _ => Err(AppError::Validation(format!(
                    "unexpected request url: {}",
                    request.url
                ))),
            }
        }

        fn download_to_path(
            &self,
            _request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            panic!("download_to_path should not be called in this test")
        }
    }

    let _guard = curseforge_api_key_guard("test-api-key");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default());
    let dependencies = provider
        .resolve_addon_dependencies(ResolveAddonDependenciesRequest {
            source: &AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: Some(777),
            },
            context: AddonProviderContext::default(),
        })
        .expect("resolve dependencies");

    assert_eq!(
        dependencies.strategy,
        AddonDependencyResolutionStrategy::MissingRequiredOnly
    );
    assert_eq!(
        dependencies.dependencies,
        vec![AddonSourceRef::CurseForgeMod {
            mod_id: 99,
            file_id: None,
        }]
    );
    assert_eq!(provider.http_client().requests.borrow().len(), 1);
}

#[test]
fn default_addon_provider_reports_dependency_resolution_capability_by_source_kind() {
    let provider = DefaultAddonProvider::with_http_client(ReqwestHttpClient::default());

    assert_eq!(
        provider.dependency_resolution_capability(&AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: None,
        }),
        AddonDependencyResolutionCapability::missing_required_only()
    );
    assert_eq!(
        provider.dependency_resolution_capability(&AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: None,
            asset_name: None,
        }),
        AddonDependencyResolutionCapability::Unsupported
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
                    r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip","size":17,"digest":"sha256:1111","updated_at":"2026-04-20T10:00:00Z"}]}"#
                }
                _ => {
                    r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/releases/v1.2.3/addon.zip","size":21,"digest":"sha256:2222","updated_at":"2026-04-21T10:00:00Z"}]}"#
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
                    r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-20T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"fileLength":17,"hashes":[{"value":"aaaa","algo":1},{"value":"bbbb","algo":2}]}}"#
                }
                _ => {
                    r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"fileLength":21,"hashes":[{"value":"cccc","algo":1},{"value":"dddd","algo":2}]}}"#
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

#[test]
fn guess_archive_name_from_url_ignores_query_string_and_fragment() {
    assert_eq!(
        super::guess_archive_name_from_url(
            "https://example.com/downloads/addon.zip?token=abc123#section",
        )
        .as_deref(),
        Some("addon.zip")
    );
}

#[test]
fn default_addon_provider_retries_failed_http_archive_downloads() {
    #[derive(Default)]
    struct FakeHttpClient {
        attempts: RefCell<usize>,
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
            let mut attempts = self.attempts.borrow_mut();
            *attempts += 1;
            if *attempts == 1 {
                return Err(AppError::Validation(
                    "transient download failure".to_string(),
                ));
            }

            std::fs::write(&request.destination, b"archive").expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_retry_policy(super::AddonProviderRetryPolicy { max_attempts: 2 });
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };

    let materialized = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: temp.path(),
            context: AddonProviderContext::default(),
        })
        .expect("materialize with retry");

    assert!(materialized.archive_path.exists());
    assert_eq!(*provider.http_client().attempts.borrow(), 2);
}

#[test]
fn default_addon_provider_forwards_download_progress_to_observer() {
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
            Ok(successful_download_response(Vec::new()))
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
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient);
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };
    let observer = FakeObserver::default();

    let materialized = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: temp.path(),
            context: AddonProviderContext::new(None, None).with_download_progress(Some(&observer)),
        })
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
fn reqwest_http_client_default_uses_bounded_timeouts() {
    let client = ReqwestHttpClient::default();

    assert_eq!(client.connect_timeout(), Duration::from_secs(10));
    assert_eq!(client.request_timeout(), Duration::from_secs(30));
}

#[test]
fn default_addon_provider_forwards_cancellation_without_retrying() {
    #[derive(Default)]
    struct FakeHttpClient {
        attempts: Cell<usize>,
        saw_cancelled: Cell<bool>,
    }

    impl HttpClient for FakeHttpClient {
        fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
            panic!("get should not be called in this test")
        }

        fn download_to_path(
            &self,
            _request: HttpDownloadRequest,
            cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.attempts.set(self.attempts.get() + 1);
            self.saw_cancelled.set(cancellation.is_cancelled());
            Err(AppError::Cancelled(
                "addon provider download cancelled".to_string(),
            ))
        }
    }

    struct AlwaysCancelled;

    impl CancellationToken for AlwaysCancelled {
        fn is_cancelled(&self) -> bool {
            true
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_retry_policy(super::AddonProviderRetryPolicy { max_attempts: 3 });
    let source = AddonSourceRef::HttpArchive {
        url: "https://example.com/addon.zip".to_string(),
    };
    let cancellation = AlwaysCancelled;

    let error = provider
        .materialize_source_ref(super::MaterializeSourceRefRequest {
            source: &source,
            stage_root: temp.path(),
            context: AddonProviderContext::new(None, Some(&cancellation)),
        })
        .expect_err("cancelled download");

    assert!(matches!(error, AppError::Cancelled(_)));
    assert_eq!(provider.http_client().attempts.get(), 1);
    assert!(provider.http_client().saw_cancelled.get());
}

#[test]
fn select_github_release_asset_requires_disambiguation() {
    let release = GitHubRelease {
        tag_name: "v1.2.3".to_string(),
        draft: false,
        prerelease: false,
        assets: vec![
            github_release_asset("a.zip", "https://example.com/a.zip"),
            github_release_asset("b.zip", "https://example.com/b.zip"),
        ],
    };

    let error = select_github_release_asset(&release, None).expect_err("ambiguous");
    assert!(error.to_string().contains("multiple `.zip` assets"));
}

#[test]
fn select_github_release_asset_matches_explicit_asset() {
    let release = GitHubRelease {
        tag_name: "v1.2.3".to_string(),
        draft: false,
        prerelease: false,
        assets: vec![
            github_release_asset("addon.zip", "https://example.com/addon.zip"),
            github_release_asset("addon.txt", "https://example.com/addon.txt"),
        ],
    };

    let asset = select_github_release_asset(&release, Some("addon.zip")).expect("asset");
    assert_eq!(asset.name, "addon.zip");
}

#[test]
fn select_github_release_prefers_latest_prerelease_when_allowed() {
    let releases = vec![
        GitHubRelease {
            tag_name: "v2.0.0-beta.1".to_string(),
            draft: false,
            prerelease: true,
            assets: vec![github_release_asset(
                "addon.zip",
                "https://example.com/beta.zip",
            )],
        },
        GitHubRelease {
            tag_name: "v1.9.9".to_string(),
            draft: false,
            prerelease: false,
            assets: vec![github_release_asset(
                "addon.zip",
                "https://example.com/stable.zip",
            )],
        },
    ];

    let release = select_github_release(&releases, true).expect("release");
    assert_eq!(release.tag_name, "v2.0.0-beta.1");
}

#[test]
fn select_latest_curseforge_file_prefers_newest_available_zip() {
    let file = select_latest_curseforge_file(
        vec![
            curseforge_file(
                1,
                "addon-old.zip",
                "2026-04-01T12:00:00Z",
                "https://example.com/old.zip",
                517,
                1,
            ),
            curseforge_file(
                2,
                "addon-new.zip",
                "2026-04-02T12:00:00Z",
                "https://example.com/new.zip",
                517,
                1,
            ),
            curseforge_file(
                3,
                "addon.txt",
                "2026-04-03T12:00:00Z",
                "https://example.com/skip.txt",
                517,
                1,
            ),
        ],
        None,
        None,
    )
    .expect("latest file");

    assert_eq!(file.id, 2);
    assert_eq!(file.file_name, "addon-new.zip");
}

#[test]
fn select_latest_curseforge_file_filters_by_version_type() {
    let file = select_latest_curseforge_file(
        vec![
            curseforge_file(
                1,
                "addon-retail.zip",
                "2026-04-01T12:00:00Z",
                "https://example.com/retail.zip",
                517,
                1,
            ),
            curseforge_file(
                2,
                "addon-classic.zip",
                "2026-04-02T12:00:00Z",
                "https://example.com/classic.zip",
                775,
                1,
            ),
        ],
        Some(517),
        None,
    )
    .expect("latest file");

    assert_eq!(file.id, 1);
}

#[test]
fn select_latest_curseforge_file_respects_release_channel_limit() {
    let file = select_latest_curseforge_file(
        vec![
            curseforge_file(
                1,
                "addon-stable.zip",
                "2026-04-01T12:00:00Z",
                "https://example.com/stable.zip",
                517,
                1,
            ),
            curseforge_file(
                2,
                "addon-beta.zip",
                "2026-04-02T12:00:00Z",
                "https://example.com/beta.zip",
                517,
                2,
            ),
            curseforge_file(
                3,
                "addon-alpha.zip",
                "2026-04-03T12:00:00Z",
                "https://example.com/alpha.zip",
                517,
                3,
            ),
        ],
        Some(517),
        Some(CurseForgeFileReleaseType::Beta),
    )
    .expect("latest allowed file");

    assert_eq!(file.id, 2);
    assert_eq!(file.file_name, "addon-beta.zip");
}

#[test]
fn select_curseforge_version_type_matches_retail_slug() {
    let version_type = select_curseforge_version_type(
        &[
            CurseForgeGameVersionType {
                id: 775,
                name: "WoW Classic".to_string(),
                slug: "wow_classic".to_string(),
            },
            CurseForgeGameVersionType {
                id: 517,
                name: "WoW Retail".to_string(),
                slug: "wow_retail".to_string(),
            },
        ],
        WowFlavor::Retail,
    )
    .expect("version type");

    assert_eq!(version_type.id, 517);
}

#[test]
fn validate_curseforge_file_rejects_missing_download_url() {
    let error = validate_curseforge_file(CurseForgeFile {
        download_url: None,
        ..curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        )
    })
    .expect_err("missing download url");

    assert!(error.to_string().contains("download URL"));
}

fn github_release_asset(name: &str, browser_download_url: &str) -> GitHubReleaseAsset {
    GitHubReleaseAsset {
        name: name.to_string(),
        browser_download_url: browser_download_url.to_string(),
        size: None,
        digest: None,
        updated_at: None,
    }
}

fn curseforge_file(
    id: u32,
    file_name: &str,
    file_date: &str,
    download_url: &str,
    game_version_type_id: u32,
    release_type: u8,
) -> CurseForgeFile {
    CurseForgeFile {
        id,
        file_name: file_name.to_string(),
        file_date: file_date.to_string(),
        download_url: Some(download_url.to_string()),
        is_available: true,
        release_type,
        dependencies: Vec::new(),
        hashes: Vec::new(),
        file_length: None,
        sortable_game_versions: vec![CurseForgeSortableGameVersion {
            game_version_type_id,
        }],
    }
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

fn curseforge_api_key_guard(value: &str) -> CurseForgeApiKeyGuard {
    static CURSEFORGE_API_KEY_ENV_MUTEX: Mutex<()> = Mutex::new(());
    let lock = CURSEFORGE_API_KEY_ENV_MUTEX
        .lock()
        .expect("curseforge api key env lock");
    let key = "HEARTHSYNC_CURSEFORGE_API_KEY";
    let previous = std::env::var_os(key);
    unsafe {
        std::env::set_var(key, value);
    }
    CurseForgeApiKeyGuard {
        key,
        previous,
        _lock: lock,
    }
}

struct CurseForgeApiKeyGuard {
    key: &'static str,
    previous: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl Drop for CurseForgeApiKeyGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}
