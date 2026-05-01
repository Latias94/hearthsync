use serde::Deserialize;

use crate::core::error::{AppError, AppResult};

mod validation;

#[cfg(test)]
mod tests;

use self::validation::{
    validate_github_download_asset, validate_github_release, validate_github_releases,
};
use super::http::{HttpClient, HttpHeader, HttpRequest};
use super::validation::RemoteArchiveValidators;

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

pub(super) fn remote_validators_for_github_asset(
    asset: &GitHubReleaseAsset,
) -> RemoteArchiveValidators {
    let mut validators = RemoteArchiveValidators {
        content_length: asset.size,
        last_modified: asset.updated_at.clone(),
        etag: None,
        sha256: None,
        sha1: None,
        md5: None,
    };

    if let Some(digest) = asset.digest.as_deref()
        && let Some(value) = digest.strip_prefix("sha256:")
    {
        validators.sha256 = Some(value.to_ascii_lowercase());
    }

    validators
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
