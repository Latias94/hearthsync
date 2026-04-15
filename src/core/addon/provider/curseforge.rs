use std::env;

use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;

use super::{AddonSearchResult, AddonSourceRef};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_ACCEPT: &str = "application/json";
const CURSEFORGE_API_KEY_ENV: &str = "HEARTHSYNC_CURSEFORGE_API_KEY";
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

pub(super) fn resolve_curseforge_file(
    mod_id: u32,
    file_id: Option<u32>,
    target_flavor: Option<WowFlavor>,
) -> AppResult<CurseForgeFile> {
    let wow_context = target_flavor
        .map(resolve_curseforge_wow_context)
        .transpose()?;
    if let Some(file_id) = file_id {
        let file = fetch_curseforge_file(mod_id, file_id)?;
        if let Some(wow_context) = &wow_context {
            ensure_curseforge_file_matches_version_type(&file, wow_context.version_type_id)?;
        }
        return validate_curseforge_file(file);
    }

    let files = fetch_curseforge_mod_files(mod_id)?;
    select_latest_curseforge_file(files, wow_context.as_ref().map(|item| item.version_type_id))
}

pub(super) fn search_curseforge_mods(
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<Vec<AddonSearchResult>> {
    let wow_context = resolve_curseforge_wow_context(flavor)?;
    let client = curseforge_client()?;
    let page_size = parse_positive_usize(limit);
    let request = client
        .get(format!("{CURSEFORGE_API_BASE}/mods/search"))
        .query(&[
            ("gameId", wow_context.game_id.to_string()),
            ("gameVersionTypeId", wow_context.version_type_id.to_string()),
            ("searchFilter", query.to_string()),
            ("pageSize", page_size.to_string()),
        ]);
    let response = send_curseforge_request(request)?;
    let payload =
        serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeSearchMod>>>(&response.text()?)?;

    Ok(payload
        .data
        .into_iter()
        .map(|mod_item| {
            let matched_index = mod_item
                .latest_files_indexes
                .iter()
                .find(|item| item.game_version_type_id == wow_context.version_type_id);
            let file_id = matched_index.map(|item| item.file_id);
            let source = AddonSourceRef::CurseForgeMod {
                mod_id: mod_item.id,
                file_id,
            };
            let install_hint = source.display_name();

            AddonSearchResult {
                provider: "curseforge",
                name: mod_item.name,
                summary: mod_item.summary,
                source,
                install_hint,
                website_url: mod_item.links.website_url,
                provider_project_id: Some(mod_item.id),
                provider_file_id: file_id,
                download_count: mod_item.download_count,
            }
        })
        .collect())
}

fn parse_positive_usize(value: usize) -> usize {
    value.max(1).min(50)
}

fn resolve_curseforge_wow_context(flavor: WowFlavor) -> AppResult<CurseForgeWowContext> {
    let game_id = find_curseforge_wow_game_id()?;
    let version_types = fetch_curseforge_game_version_types(game_id)?;
    let version_type = select_curseforge_version_type(&version_types, flavor)?;

    Ok(CurseForgeWowContext {
        game_id,
        version_type_id: version_type.id,
    })
}

fn find_curseforge_wow_game_id() -> AppResult<u32> {
    let client = curseforge_client()?;
    let mut index = 0usize;

    loop {
        let url = format!("{CURSEFORGE_API_BASE}/games?index={index}&pageSize=50");
        let response = send_curseforge_request(client.get(url))?;
        let payload = serde_json::from_str::<CurseForgePaginatedResponse<Vec<CurseForgeGame>>>(
            &response.text()?,
        )?;
        let games = payload.data;
        if let Some(game) = games.iter().find(|game| is_world_of_warcraft_game(game)) {
            return Ok(game.id);
        }
        if games.len() < 50 {
            break;
        }
        index += 50;
    }

    Err(AppError::NotFound(
        "CurseForge game `World of Warcraft` was not found for the current API key".to_string(),
    ))
}

fn fetch_curseforge_game_version_types(game_id: u32) -> AppResult<Vec<CurseForgeGameVersionType>> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/games/{game_id}/version-types");
    let response = send_curseforge_request(client.get(url))?;
    let payload = serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeGameVersionType>>>(
        &response.text()?,
    )?;
    Ok(payload.data)
}

pub(super) fn select_curseforge_version_type(
    version_types: &[CurseForgeGameVersionType],
    flavor: WowFlavor,
) -> AppResult<CurseForgeGameVersionType> {
    let candidates = curseforge_version_type_candidates(flavor);
    version_types
        .iter()
        .find(|version_type| {
            let slug = version_type.slug.to_ascii_lowercase();
            let name = version_type.name.to_ascii_lowercase();
            candidates.iter().any(|candidate| {
                slug == *candidate
                    || slug.contains(candidate)
                    || name == *candidate
                    || name.contains(candidate)
            })
        })
        .cloned()
        .ok_or_else(|| {
            AppError::Validation(format!(
                "CurseForge version type for flavor `{}` was not found. Available version types: {}",
                flavor.as_str(),
                version_types
                    .iter()
                    .map(|item| format!("{}({})", item.slug, item.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

fn curseforge_version_type_candidates(flavor: WowFlavor) -> &'static [&'static str] {
    match flavor {
        WowFlavor::Retail => &["wow_retail", "retail"],
        WowFlavor::Classic => &["wow_classic", "classic"],
        WowFlavor::ClassicEra => &["wow_classic_era", "classic_era", "classic era"],
        WowFlavor::Ptr => &["wow_ptr", "ptr"],
        WowFlavor::Beta => &["wow_beta", "beta"],
        WowFlavor::Xptr => &["wow_xptr", "xptr"],
    }
}

fn is_world_of_warcraft_game(game: &CurseForgeGame) -> bool {
    let name = game.name.to_ascii_lowercase();
    let slug = game.slug.to_ascii_lowercase();
    name == "world of warcraft"
        || slug == "world-of-warcraft"
        || slug == "world_of_warcraft"
        || (name.contains("world") && name.contains("warcraft"))
}

fn curseforge_client() -> AppResult<Client> {
    let api_key = env::var(CURSEFORGE_API_KEY_ENV).map_err(|_| {
        AppError::Validation(format!(
            "CurseForge provider requires environment variable `{CURSEFORGE_API_KEY_ENV}`"
        ))
    })?;
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static(CURSEFORGE_ACCEPT));
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(&api_key).map_err(|error| AppError::Validation(error.to_string()))?,
    );

    Ok(Client::builder().default_headers(headers).build()?)
}

fn send_curseforge_request(
    request: reqwest::blocking::RequestBuilder,
) -> AppResult<reqwest::blocking::Response> {
    let response = request.send()?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let message = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => format!(
            "CurseForge request was rejected with {}. Check `{CURSEFORGE_API_KEY_ENV}` and ensure the API key is valid for the official CurseForge REST API.",
            status
        ),
        _ => format!("CurseForge request failed with HTTP status {status}"),
    };

    Err(AppError::Validation(message))
}

fn fetch_curseforge_mod_files(mod_id: u32) -> AppResult<Vec<CurseForgeFile>> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files");
    let response = send_curseforge_request(client.get(url))?;
    let payload =
        serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeFile>>>(&response.text()?)?;
    Ok(payload.data)
}

fn fetch_curseforge_file(mod_id: u32, file_id: u32) -> AppResult<CurseForgeFile> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files/{file_id}");
    let response = send_curseforge_request(client.get(url))?;
    let payload = serde_json::from_str::<CurseForgeApiResponse<CurseForgeFile>>(&response.text()?)?;
    Ok(payload.data)
}

pub(super) fn select_latest_curseforge_file(
    files: Vec<CurseForgeFile>,
    version_type_id: Option<u32>,
) -> AppResult<CurseForgeFile> {
    let mut candidates = files
        .into_iter()
        .filter(|file| file.is_available)
        .filter(|file| file.file_name.ends_with(".zip"))
        .filter(|file| {
            version_type_id.is_none_or(|version_type_id| {
                file_matches_curseforge_version_type(file, version_type_id)
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.file_date.cmp(&left.file_date));

    let Some(file) = candidates.into_iter().next() else {
        return Err(AppError::Validation(match version_type_id {
            Some(version_type_id) => format!(
                "CurseForge mod does not expose an available `.zip` file for version type `{version_type_id}`"
            ),
            None => "CurseForge mod does not expose an available `.zip` file".to_string(),
        }));
    };

    validate_curseforge_file(file)
}

fn ensure_curseforge_file_matches_version_type(
    file: &CurseForgeFile,
    version_type_id: u32,
) -> AppResult<()> {
    if file_matches_curseforge_version_type(file, version_type_id) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "CurseForge file `{}` does not match target version type `{version_type_id}`",
            file.id
        )))
    }
}

fn file_matches_curseforge_version_type(file: &CurseForgeFile, version_type_id: u32) -> bool {
    file.sortable_game_versions
        .iter()
        .any(|item| item.game_version_type_id == version_type_id)
}

pub(super) fn validate_curseforge_file(file: CurseForgeFile) -> AppResult<CurseForgeFile> {
    if !file.is_available {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` is not available for download",
            file.id
        )));
    }
    if !file.file_name.ends_with(".zip") {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` is not a `.zip` archive",
            file.file_name
        )));
    }
    if file.download_url.is_none() {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` does not provide a download URL",
            file.id
        )));
    }

    Ok(file)
}

#[derive(Debug, Deserialize)]
struct CurseForgeApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgePaginatedResponse<T> {
    data: T,
}

#[derive(Debug, Clone)]
struct CurseForgeWowContext {
    game_id: u32,
    version_type_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeGame {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeGameVersionType {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeFile {
    pub(super) id: u32,
    pub(super) file_name: String,
    pub(super) file_date: String,
    pub(super) download_url: Option<String>,
    pub(super) is_available: bool,
    #[serde(default)]
    pub(super) sortable_game_versions: Vec<CurseForgeSortableGameVersion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CurseForgeSortableGameVersion {
    pub(super) game_version_type_id: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeSearchMod {
    id: u32,
    name: String,
    summary: Option<String>,
    download_count: u64,
    #[serde(default)]
    latest_files_indexes: Vec<CurseForgeFileIndex>,
    links: CurseForgeSearchModLinks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeSearchModLinks {
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFileIndex {
    file_id: u32,
    game_version_type_id: u32,
}
