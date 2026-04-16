use std::env;

use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};

use super::model::{
    CurseForgeApiResponse, CurseForgeFile, CurseForgeGame, CurseForgeGameVersionType,
    CurseForgePaginatedResponse, CurseForgeSearchMod, CurseForgeWowContext,
};
use super::select::{is_world_of_warcraft_game, select_curseforge_version_type};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_ACCEPT: &str = "application/json";
const CURSEFORGE_API_KEY_ENV: &str = "HEARTHSYNC_CURSEFORGE_API_KEY";
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

pub(super) fn resolve_curseforge_wow_context(flavor: WowFlavor) -> AppResult<CurseForgeWowContext> {
    let game_id = find_curseforge_wow_game_id()?;
    let version_types = fetch_curseforge_game_version_types(game_id)?;
    let version_type = select_curseforge_version_type(&version_types, flavor)?;

    Ok(CurseForgeWowContext {
        game_id,
        version_type_id: version_type.id,
    })
}

pub(super) fn fetch_curseforge_mod_files(mod_id: u32) -> AppResult<Vec<CurseForgeFile>> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files");
    let response = send_curseforge_request(client.get(url))?;
    let payload =
        serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeFile>>>(&response.text()?)?;
    Ok(payload.data)
}

pub(super) fn fetch_curseforge_file(mod_id: u32, file_id: u32) -> AppResult<CurseForgeFile> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files/{file_id}");
    let response = send_curseforge_request(client.get(url))?;
    let payload = serde_json::from_str::<CurseForgeApiResponse<CurseForgeFile>>(&response.text()?)?;
    Ok(payload.data)
}

pub(super) fn search_curseforge_mods(
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<(CurseForgeWowContext, Vec<CurseForgeSearchMod>)> {
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

    Ok((wow_context, payload.data))
}

fn parse_positive_usize(value: usize) -> usize {
    value.max(1).min(50)
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
