use std::env;

mod validation;

#[cfg(test)]
mod tests;

use self::validation::{
    validate_curseforge_game_version_types, validate_curseforge_games,
    validate_curseforge_search_mods,
};
use super::file_validation::validate_curseforge_file_metadata;
use super::model::{
    CurseForgeApiResponse, CurseForgeFile, CurseForgeGame, CurseForgeGameVersionType,
    CurseForgePaginatedResponse, CurseForgeSearchMod, CurseForgeWowContext,
};
use super::select::{is_world_of_warcraft_game, select_curseforge_version_type};
use crate::core::addon::provider::http::{HttpClient, HttpHeader, HttpRequest};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_ACCEPT: &str = "application/json";
const HEARTHSYNC_CURSEFORGE_API_KEY_ENV: &str = "HEARTHSYNC_CURSEFORGE_API_KEY";
const STANDARD_CURSEFORGE_API_KEY_ENV: &str = "CURSEFORGE_API_KEY";
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

pub(super) fn resolve_curseforge_wow_context_with_client(
    client: &impl HttpClient,
    flavor: WowFlavor,
) -> AppResult<CurseForgeWowContext> {
    let game_id = find_curseforge_wow_game_id(client)?;
    let version_types = fetch_curseforge_game_version_types(client, game_id)?;
    let version_type = select_curseforge_version_type(&version_types, flavor)?;

    Ok(CurseForgeWowContext {
        game_id,
        version_type_id: version_type.id,
    })
}

pub(super) fn fetch_curseforge_mod_files_with_client(
    client: &impl HttpClient,
    mod_id: u32,
) -> AppResult<Vec<CurseForgeFile>> {
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files");
    let response = send_curseforge_request(
        client,
        HttpRequest::new(url).with_headers(curseforge_headers()?),
    )?;
    let payload = serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeFile>>>(&response)?;
    validate_curseforge_files_metadata(&payload.data)?;
    Ok(payload.data)
}

pub(super) fn fetch_curseforge_file_with_client(
    client: &impl HttpClient,
    mod_id: u32,
    file_id: u32,
) -> AppResult<CurseForgeFile> {
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files/{file_id}");
    let response = send_curseforge_request(
        client,
        HttpRequest::new(url).with_headers(curseforge_headers()?),
    )?;
    let payload = serde_json::from_str::<CurseForgeApiResponse<CurseForgeFile>>(&response)?;
    validate_curseforge_file_metadata(&payload.data)?;
    Ok(payload.data)
}

pub(super) fn search_curseforge_mods_with_client(
    client: &impl HttpClient,
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<(CurseForgeWowContext, Vec<CurseForgeSearchMod>)> {
    let wow_context = resolve_curseforge_wow_context_with_client(client, flavor)?;
    let page_size = parse_positive_usize(limit);
    let request = HttpRequest::new(format!("{CURSEFORGE_API_BASE}/mods/search"))
        .with_headers(curseforge_headers()?)
        .with_query(vec![
            ("gameId".to_string(), wow_context.game_id.to_string()),
            (
                "gameVersionTypeId".to_string(),
                wow_context.version_type_id.to_string(),
            ),
            ("searchFilter".to_string(), query.to_string()),
            ("pageSize".to_string(), page_size.to_string()),
        ]);
    let response = send_curseforge_request(client, request)?;
    let payload =
        serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeSearchMod>>>(&response)?;
    validate_curseforge_search_mods(&payload.data)?;

    Ok((wow_context, payload.data))
}

fn parse_positive_usize(value: usize) -> usize {
    value.clamp(1, 50)
}

fn find_curseforge_wow_game_id(client: &impl HttpClient) -> AppResult<u32> {
    let mut index = 0usize;

    loop {
        let response = send_curseforge_request(
            client,
            HttpRequest::new(format!("{CURSEFORGE_API_BASE}/games"))
                .with_headers(curseforge_headers()?)
                .with_query(vec![
                    ("index".to_string(), index.to_string()),
                    ("pageSize".to_string(), "50".to_string()),
                ]),
        )?;
        let payload =
            serde_json::from_str::<CurseForgePaginatedResponse<Vec<CurseForgeGame>>>(&response)?;
        let games = payload.data;
        validate_curseforge_games(&games)?;
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

fn fetch_curseforge_game_version_types(
    client: &impl HttpClient,
    game_id: u32,
) -> AppResult<Vec<CurseForgeGameVersionType>> {
    let url = format!("{CURSEFORGE_API_BASE}/games/{game_id}/version-types");
    let response = send_curseforge_request(
        client,
        HttpRequest::new(url).with_headers(curseforge_headers()?),
    )?;
    let payload =
        serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeGameVersionType>>>(&response)?;
    validate_curseforge_game_version_types(&payload.data)?;
    Ok(payload.data)
}

fn curseforge_headers() -> AppResult<Vec<HttpHeader>> {
    let api_key = curseforge_api_key()?;
    Ok(vec![
        HttpHeader {
            name: "Accept".to_string(),
            value: CURSEFORGE_ACCEPT.to_string(),
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: USER_AGENT_VALUE.to_string(),
        },
        HttpHeader {
            name: "x-api-key".to_string(),
            value: api_key,
        },
    ])
}

fn curseforge_api_key() -> AppResult<String> {
    env::var(HEARTHSYNC_CURSEFORGE_API_KEY_ENV)
        .or_else(|_| env::var(STANDARD_CURSEFORGE_API_KEY_ENV))
        .map_err(|_| {
            AppError::Validation(format!(
                "CurseForge provider requires environment variable `{HEARTHSYNC_CURSEFORGE_API_KEY_ENV}` or `{STANDARD_CURSEFORGE_API_KEY_ENV}`"
            ))
        })
}

fn send_curseforge_request(client: &impl HttpClient, request: HttpRequest) -> AppResult<String> {
    let response = client.get(request)?;
    if response.is_success() {
        return Ok(response.body);
    }

    let message = match response.status_code {
        401 | 403 => format!(
            "CurseForge request was rejected with {}. Check `{HEARTHSYNC_CURSEFORGE_API_KEY_ENV}` or `{STANDARD_CURSEFORGE_API_KEY_ENV}` and ensure the API key is valid for the official CurseForge REST API.",
            response.status_code
        ),
        _ => format!(
            "CurseForge request failed with HTTP status {}",
            response.status_code
        ),
    };

    Err(AppError::Validation(message))
}

fn validate_curseforge_files_metadata(files: &[CurseForgeFile]) -> AppResult<()> {
    for file in files {
        validate_curseforge_file_metadata(file)?;
    }

    Ok(())
}
