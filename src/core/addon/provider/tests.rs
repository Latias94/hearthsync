use std::cell::{Cell, RefCell};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::curseforge::{
    CurseForgeFile, CurseForgeGameVersionType, CurseForgeSortableGameVersion,
    select_curseforge_version_type, select_latest_curseforge_file, validate_curseforge_file,
};
use super::github::fetch_github_release_with_client;
use super::github::{GitHubRelease, GitHubReleaseAsset, select_github_release_asset};
use super::http::{HttpClient, HttpDownloadRequest, HttpRequest, HttpResponse, ReqwestHttpClient};
use super::parse::{parse_curseforge_source, parse_github_source};
use super::{AddonProvider, AddonProviderContext, AddonSourceRef, DefaultAddonProvider};
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
                body: r#"{"assets":[{"name":"addon.zip","browser_download_url":"https://example.com/addon.zip"}]}"#.to_string(),
            })
        }

        fn download_to_path(
            &self,
            _request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
        ) -> AppResult<()> {
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
                body: r#"{"assets":[{"name":"addon.zip","browser_download_url":"https://example.com/addon.zip"}]}"#.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
        ) -> AppResult<()> {
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
            Ok(())
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
fn default_addon_provider_reuses_download_cache_for_http_archives() {
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
        ) -> AppResult<()> {
            self.downloads.borrow_mut().push(request.clone());
            std::fs::write(&request.destination, b"archive").expect("archive file");
            Ok(())
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
        ) -> AppResult<()> {
            let mut attempts = self.attempts.borrow_mut();
            *attempts += 1;
            if *attempts == 1 {
                return Err(AppError::Validation(
                    "transient download failure".to_string(),
                ));
            }

            std::fs::write(&request.destination, b"archive").expect("archive file");
            Ok(())
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
        ) -> AppResult<()> {
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
            context: AddonProviderContext {
                target_flavor: None,
                cancellation: Some(&cancellation),
            },
        })
        .expect_err("cancelled download");

    assert!(matches!(error, AppError::Cancelled(_)));
    assert_eq!(provider.http_client().attempts.get(), 1);
    assert!(provider.http_client().saw_cancelled.get());
}

#[test]
fn select_github_release_asset_requires_disambiguation() {
    let release = GitHubRelease {
        assets: vec![
            GitHubReleaseAsset {
                name: "a.zip".to_string(),
                browser_download_url: "https://example.com/a.zip".to_string(),
            },
            GitHubReleaseAsset {
                name: "b.zip".to_string(),
                browser_download_url: "https://example.com/b.zip".to_string(),
            },
        ],
    };

    let error = select_github_release_asset(&release, None).expect_err("ambiguous");
    assert!(error.to_string().contains("multiple `.zip` assets"));
}

#[test]
fn select_github_release_asset_matches_explicit_asset() {
    let release = GitHubRelease {
        assets: vec![
            GitHubReleaseAsset {
                name: "addon.zip".to_string(),
                browser_download_url: "https://example.com/addon.zip".to_string(),
            },
            GitHubReleaseAsset {
                name: "addon.txt".to_string(),
                browser_download_url: "https://example.com/addon.txt".to_string(),
            },
        ],
    };

    let asset = select_github_release_asset(&release, Some("addon.zip")).expect("asset");
    assert_eq!(asset.name, "addon.zip");
}

#[test]
fn select_latest_curseforge_file_prefers_newest_available_zip() {
    let file = select_latest_curseforge_file(
        vec![
            CurseForgeFile {
                id: 1,
                file_name: "addon-old.zip".to_string(),
                file_date: "2026-04-01T12:00:00Z".to_string(),
                download_url: Some("https://example.com/old.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
            CurseForgeFile {
                id: 2,
                file_name: "addon-new.zip".to_string(),
                file_date: "2026-04-02T12:00:00Z".to_string(),
                download_url: Some("https://example.com/new.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
            CurseForgeFile {
                id: 3,
                file_name: "addon.txt".to_string(),
                file_date: "2026-04-03T12:00:00Z".to_string(),
                download_url: Some("https://example.com/skip.txt".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
        ],
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
            CurseForgeFile {
                id: 1,
                file_name: "addon-retail.zip".to_string(),
                file_date: "2026-04-01T12:00:00Z".to_string(),
                download_url: Some("https://example.com/retail.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 517,
                }],
            },
            CurseForgeFile {
                id: 2,
                file_name: "addon-classic.zip".to_string(),
                file_date: "2026-04-02T12:00:00Z".to_string(),
                download_url: Some("https://example.com/classic.zip".to_string()),
                is_available: true,
                sortable_game_versions: vec![CurseForgeSortableGameVersion {
                    game_version_type_id: 775,
                }],
            },
        ],
        Some(517),
    )
    .expect("latest file");

    assert_eq!(file.id, 1);
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
        id: 1,
        file_name: "addon.zip".to_string(),
        file_date: "2026-04-02T12:00:00Z".to_string(),
        download_url: None,
        is_available: true,
        sortable_game_versions: Vec::new(),
    })
    .expect_err("missing download url");

    assert!(error.to_string().contains("download URL"));
}
