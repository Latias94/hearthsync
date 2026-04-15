use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;

use crate::core::error::{AppError, AppResult};

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

pub(super) fn fetch_github_release(
    owner: &str,
    repo: &str,
    tag: Option<&str>,
) -> AppResult<GitHubRelease> {
    let client = github_client()?;
    let url = match tag {
        Some(tag) => format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases/tags/{tag}"),
        None => format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases/latest"),
    };
    let response = client.get(url).send()?.error_for_status()?;
    Ok(serde_json::from_str(&response.text()?)?)
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

fn github_client() -> AppResult<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static(GITHUB_ACCEPT));
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static(GITHUB_API_VERSION),
    );

    Ok(Client::builder().default_headers(headers).build()?)
}

#[derive(Debug, Deserialize)]
pub(super) struct GitHubRelease {
    pub(super) assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GitHubReleaseAsset {
    pub(super) name: String,
    pub(super) browser_download_url: String,
}
