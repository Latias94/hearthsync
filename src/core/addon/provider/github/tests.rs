use std::cell::RefCell;

use super::super::http::{
    HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse, HttpResponse,
};
use super::*;
use crate::core::task::CancellationToken;

#[test]
fn fetch_github_release_with_client_uses_http_port() {
    let client = JsonResponseHttpClient::new(
        r#"{"tag_name":"v1.2.3","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/addon.zip"}]}"#,
    );
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
    let client = JsonResponseHttpClient::new(
        r#"[{"tag_name":"v1.2.3","prerelease":true,"assets":[{"name":"addon.zip","browser_download_url":"https://example.com/addon.zip"}]}]"#,
    );
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
fn fetch_github_release_with_client_rejects_invalid_release_contracts() {
    let client = JsonResponseHttpClient::new(
        r#"{"tag_name":" ","assets":[{"name":"addon.zip","browser_download_url":"https://example.com/addon.zip"}]}"#,
    );

    let error = fetch_github_release_with_client(&client, "owner", "repo", Some("v1.2.3"))
        .expect_err("invalid release");

    assert!(
        error
            .to_string()
            .contains("GitHub release tag name must not be empty")
    );
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
fn select_github_release_asset_rejects_non_portable_asset_names() {
    let release = GitHubRelease {
        tag_name: "v1.2.3".to_string(),
        draft: false,
        prerelease: false,
        assets: vec![github_release_asset(
            "bad/name.zip",
            "https://example.com/addon.zip",
        )],
    };

    let error = select_github_release_asset(&release, None).expect_err("unsafe asset name");
    assert!(
        error
            .to_string()
            .contains("invalid GitHub release asset name")
    );
}

#[test]
fn select_github_release_asset_rejects_invalid_download_url() {
    let release = GitHubRelease {
        tag_name: "v1.2.3".to_string(),
        draft: false,
        prerelease: false,
        assets: vec![github_release_asset(
            "addon.zip",
            "ftp://example.com/addon.zip",
        )],
    };

    let error = select_github_release_asset(&release, None).expect_err("invalid download url");
    assert!(error.to_string().contains("download URL must start with"));
}

#[test]
fn select_github_release_asset_rejects_invalid_remote_validator_metadata() {
    let cases = vec![
        (
            GitHubReleaseAsset {
                size: Some(0),
                ..github_release_asset("addon.zip", "https://example.com/addon.zip")
            },
            "size must be greater than zero",
        ),
        (
            GitHubReleaseAsset {
                digest: Some("sha256:abc".to_string()),
                ..github_release_asset("addon.zip", "https://example.com/addon.zip")
            },
            "digest must be a 64-character SHA-256 hex digest",
        ),
        (
            GitHubReleaseAsset {
                digest: Some("sha1:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()),
                ..github_release_asset("addon.zip", "https://example.com/addon.zip")
            },
            "digest must use the `sha256:` prefix",
        ),
        (
            GitHubReleaseAsset {
                updated_at: Some("2026-04-02 12:00:00".to_string()),
                ..github_release_asset("addon.zip", "https://example.com/addon.zip")
            },
            "updated_at must be an RFC 3339 timestamp",
        ),
    ];

    for (asset, expected_message) in cases {
        let release = GitHubRelease {
            tag_name: "v1.2.3".to_string(),
            draft: false,
            prerelease: false,
            assets: vec![asset],
        };

        let error = select_github_release_asset(&release, None)
            .expect_err("invalid remote validator metadata");
        assert!(
            error.to_string().contains(expected_message),
            "expected `{}` in `{}`",
            expected_message,
            error
        );
    }
}

#[test]
fn select_github_release_asset_rejects_explicit_non_zip_asset() {
    let release = GitHubRelease {
        tag_name: "v1.2.3".to_string(),
        draft: false,
        prerelease: false,
        assets: vec![github_release_asset(
            "addon.txt",
            "https://example.com/addon.txt",
        )],
    };

    let error =
        select_github_release_asset(&release, Some("addon.txt")).expect_err("non-zip asset");
    assert!(error.to_string().contains("is not a `.zip` archive"));
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

fn github_release_asset(name: &str, browser_download_url: &str) -> GitHubReleaseAsset {
    GitHubReleaseAsset {
        name: name.to_string(),
        browser_download_url: browser_download_url.to_string(),
        size: None,
        digest: None,
        updated_at: None,
    }
}

struct JsonResponseHttpClient<'a> {
    body: &'a str,
    requests: RefCell<Vec<HttpRequest>>,
}

impl<'a> JsonResponseHttpClient<'a> {
    fn new(body: &'a str) -> Self {
        Self {
            body,
            requests: RefCell::new(Vec::new()),
        }
    }
}

impl HttpClient for JsonResponseHttpClient<'_> {
    fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
        self.requests.borrow_mut().push(request);
        Ok(HttpResponse {
            status_code: 200,
            body: self.body.to_string(),
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
