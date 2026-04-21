use serde::Deserialize;

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
    Ok(serde_json::from_str(&response.body)?)
}

pub(super) fn select_github_release_asset<'a>(
    release: &'a GitHubRelease,
    requested_asset_name: Option<&str>,
) -> AppResult<&'a GitHubReleaseAsset> {
    if let Some(requested_asset_name) = requested_asset_name {
        return release
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
            });
    }

    let zip_assets = release
        .assets
        .iter()
        .filter(|asset| asset.name.ends_with(".zip"))
        .collect::<Vec<_>>();
    match zip_assets.len() {
        0 => Err(AppError::Validation(
            "GitHub release does not contain a `.zip` asset".to_string(),
        )),
        1 => Ok(zip_assets[0]),
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

#[derive(Debug, Deserialize)]
pub(super) struct GitHubRelease {
    pub(super) tag_name: String,
    pub(super) assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GitHubReleaseAsset {
    pub(super) name: String,
    pub(super) browser_download_url: String,
}
