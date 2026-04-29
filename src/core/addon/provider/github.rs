use std::collections::BTreeSet;

use serde::Deserialize;

use crate::core::archive_path::validate_portable_path_segment;
use crate::core::boundary_validation::validate_http_url;
use crate::core::error::{AppError, AppResult};

use super::http::{HttpClient, HttpHeader, HttpRequest};

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

pub(super) fn fetch_github_release_with_client(
    client: &impl HttpClient,
    owner: &str,
    repo: &str,
    tag: Option<&str>,
) -> AppResult<GitHubRelease> {
    let url = match tag {
        Some(tag) => format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases/tags/{tag}"),
        None => format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases/latest"),
    };
    let response = client.get(HttpRequest::new(url).with_headers(github_headers()))?;
    if !response.is_success() {
        return Err(AppError::Validation(format!(
            "GitHub request failed with HTTP status {}",
            response.status_code
        )));
    }
    let release = serde_json::from_str(&response.body)?;
    validate_github_release(&release)?;
    Ok(release)
}

pub(super) fn fetch_github_releases_with_client(
    client: &impl HttpClient,
    owner: &str,
    repo: &str,
) -> AppResult<Vec<GitHubRelease>> {
    let url = format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases");
    let response = client.get(HttpRequest::new(url).with_headers(github_headers()))?;
    if !response.is_success() {
        return Err(AppError::Validation(format!(
            "GitHub request failed with HTTP status {}",
            response.status_code
        )));
    }
    let releases: Vec<GitHubRelease> = serde_json::from_str(&response.body)?;
    validate_github_releases(&releases)?;
    Ok(releases)
}

pub(super) fn select_github_release(
    releases: &[GitHubRelease],
    allow_prerelease: bool,
) -> AppResult<&GitHubRelease> {
    releases
        .iter()
        .find(|release| !release.draft && (allow_prerelease || !release.prerelease))
        .ok_or_else(|| {
            AppError::Validation(if allow_prerelease {
                "GitHub repository does not expose a published release".to_string()
            } else {
                "GitHub repository does not expose a published stable release".to_string()
            })
        })
}

pub(super) fn select_github_release_asset<'a>(
    release: &'a GitHubRelease,
    requested_asset_name: Option<&str>,
) -> AppResult<&'a GitHubReleaseAsset> {
    validate_github_release(release)?;

    if let Some(requested_asset_name) = requested_asset_name {
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name.eq_ignore_ascii_case(requested_asset_name))
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "GitHub release asset `{requested_asset_name}` not found; available assets: {}",
                    release
                        .assets
                        .iter()
                        .map(|asset| asset.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })?;
        validate_github_download_asset(asset)?;
        return Ok(asset);
    }

    let zip_assets = release
        .assets
        .iter()
        .filter(|asset| is_zip_asset_name(&asset.name))
        .collect::<Vec<_>>();
    match zip_assets.len() {
        0 => Err(AppError::Validation(
            "GitHub release does not contain a `.zip` asset".to_string(),
        )),
        1 => {
            validate_github_download_asset(zip_assets[0])?;
            Ok(zip_assets[0])
        }
        _ => Err(AppError::Validation(format!(
            "GitHub release has multiple `.zip` assets; specify one with `github:owner/repo[#asset.zip]`: {}",
            zip_assets
                .into_iter()
                .map(|asset| asset.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn validate_github_releases(releases: &[GitHubRelease]) -> AppResult<()> {
    for release in releases {
        validate_github_release(release)?;
    }

    Ok(())
}

fn validate_github_release(release: &GitHubRelease) -> AppResult<()> {
    if release.tag_name.trim().is_empty() {
        return Err(AppError::Validation(
            "GitHub release tag name must not be empty".to_string(),
        ));
    }

    let mut asset_names = BTreeSet::new();
    for asset in &release.assets {
        validate_github_release_asset(asset)?;
        let normalized_name = asset.name.to_ascii_lowercase();
        if !asset_names.insert(normalized_name) {
            return Err(AppError::Validation(format!(
                "GitHub release asset `{}` is duplicated under case-insensitive comparison",
                asset.name
            )));
        }
    }

    Ok(())
}

fn validate_github_download_asset(asset: &GitHubReleaseAsset) -> AppResult<()> {
    validate_github_release_asset(asset)?;
    if !is_zip_asset_name(&asset.name) {
        return Err(AppError::Validation(format!(
            "GitHub release asset `{}` is not a `.zip` archive",
            asset.name
        )));
    }

    Ok(())
}

fn validate_github_release_asset(asset: &GitHubReleaseAsset) -> AppResult<()> {
    validate_portable_path_segment(&asset.name, "GitHub release asset")?;
    validate_http_url(
        &asset.browser_download_url,
        &format!("GitHub release asset `{}` download URL", asset.name),
    )?;

    Ok(())
}

fn is_zip_asset_name(asset_name: &str) -> bool {
    asset_name.to_ascii_lowercase().ends_with(".zip")
}

pub(super) fn github_headers() -> Vec<HttpHeader> {
    vec![
        HttpHeader {
            name: "Accept".to_string(),
            value: GITHUB_ACCEPT.to_string(),
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: USER_AGENT_VALUE.to_string(),
        },
        HttpHeader {
            name: "X-GitHub-Api-Version".to_string(),
            value: GITHUB_API_VERSION.to_string(),
        },
    ]
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct GitHubRelease {
    pub(super) tag_name: String,
    #[serde(default)]
    pub(super) draft: bool,
    #[serde(default)]
    pub(super) prerelease: bool,
    pub(super) assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct GitHubReleaseAsset {
    pub(super) name: String,
    pub(super) browser_download_url: String,
    #[serde(default)]
    pub(super) size: Option<u64>,
    #[serde(default)]
    pub(super) digest: Option<String>,
    #[serde(default)]
    pub(super) updated_at: Option<String>,
}
