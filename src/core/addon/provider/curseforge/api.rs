use std::collections::BTreeSet;
use std::env;

use super::file_validation::validate_curseforge_file_metadata;
use super::model::{
    CurseForgeApiResponse, CurseForgeFile, CurseForgeGame, CurseForgeGameVersionType,
    CurseForgePaginatedResponse, CurseForgeSearchMod, CurseForgeWowContext,
};
use super::select::{is_world_of_warcraft_game, select_curseforge_version_type};
use crate::core::addon::provider::http::{HttpClient, HttpHeader, HttpRequest};
use crate::core::boundary_validation::validate_http_url;
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

fn validate_curseforge_games(games: &[CurseForgeGame]) -> AppResult<()> {
    let mut game_ids = BTreeSet::new();
    for game in games {
        if game.id == 0 {
            return Err(AppError::Validation(
                "CurseForge game id must be greater than zero".to_string(),
            ));
        }
        validate_required_provider_text(game.id, "CurseForge game name", &game.name)?;
        validate_required_provider_text(game.id, "CurseForge game slug", &game.slug)?;
        if !game_ids.insert(game.id) {
            return Err(AppError::Validation(format!(
                "CurseForge games response declares duplicate game id `{}`",
                game.id
            )));
        }
    }

    Ok(())
}

fn validate_curseforge_game_version_types(
    version_types: &[CurseForgeGameVersionType],
) -> AppResult<()> {
    let mut version_type_ids = BTreeSet::new();
    for version_type in version_types {
        if version_type.id == 0 {
            return Err(AppError::Validation(
                "CurseForge game version type id must be greater than zero".to_string(),
            ));
        }
        validate_required_provider_text(
            version_type.id,
            "CurseForge game version type name",
            &version_type.name,
        )?;
        validate_required_provider_text(
            version_type.id,
            "CurseForge game version type slug",
            &version_type.slug,
        )?;
        if !version_type_ids.insert(version_type.id) {
            return Err(AppError::Validation(format!(
                "CurseForge game version types response declares duplicate version type id `{}`",
                version_type.id
            )));
        }
    }

    Ok(())
}

fn validate_curseforge_search_mods(mods: &[CurseForgeSearchMod]) -> AppResult<()> {
    for mod_item in mods {
        validate_curseforge_search_mod(mod_item)?;
    }

    Ok(())
}

fn validate_curseforge_search_mod(mod_item: &CurseForgeSearchMod) -> AppResult<()> {
    if mod_item.id == 0 {
        return Err(AppError::Validation(
            "CurseForge search result mod id must be greater than zero".to_string(),
        ));
    }
    validate_required_provider_text(mod_item.id, "CurseForge search result name", &mod_item.name)?;
    if let Some(website_url) = &mod_item.links.website_url {
        validate_http_url(
            website_url,
            &format!("CurseForge search result `{}` website URL", mod_item.id),
        )?;
    }
    for file_index in &mod_item.latest_files_indexes {
        if file_index.file_id == 0 {
            return Err(AppError::Validation(format!(
                "CurseForge search result `{}` latest file index file id must be greater than zero",
                mod_item.id
            )));
        }
        if file_index.game_version_type_id == 0 {
            return Err(AppError::Validation(format!(
                "CurseForge search result `{}` latest file index game version type id must be greater than zero",
                mod_item.id
            )));
        }
    }

    Ok(())
}

fn validate_required_provider_text(id: u32, field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "{field} `{id}` must not be empty"
        )));
    }
    if value.trim() != value {
        return Err(AppError::Validation(format!(
            "{field} `{id}` must not have surrounding whitespace"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::super::super::test_support::curseforge_api_key_guard;
    use super::*;
    use crate::core::addon::provider::http::{
        HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse, HttpResponse,
    };
    use crate::core::task::CancellationToken;

    #[test]
    fn fetch_curseforge_mod_files_with_client_returns_validated_files() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let client = SingleRouteHttpClient::new(
            "https://api.curseforge.com/v1/mods/42/files",
            r#"{"data":[{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"releaseType":1}]}"#,
        );

        let files = fetch_curseforge_mod_files_with_client(&client, 42).expect("mod files");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].id, 777);
        assert_eq!(files[0].file_name, "addon.zip");
        assert!(
            client.requests.borrow()[0]
                .headers
                .iter()
                .any(|header| header.name == "x-api-key" && header.value == "test-api-key")
        );
    }

    #[test]
    fn fetch_curseforge_mod_files_with_client_rejects_invalid_file_contracts() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let client = SingleRouteHttpClient::new(
            "https://api.curseforge.com/v1/mods/42/files",
            r#"{"data":[{"id":777,"fileName":"bad/name.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"releaseType":1}]}"#,
        );

        let error =
            fetch_curseforge_mod_files_with_client(&client, 42).expect_err("invalid file metadata");

        assert!(error.to_string().contains("invalid CurseForge file name"));
    }

    #[test]
    fn fetch_curseforge_file_with_client_returns_validated_file() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let client = SingleRouteHttpClient::new(
            "https://api.curseforge.com/v1/mods/42/files/777",
            r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"releaseType":1}}"#,
        );

        let file = fetch_curseforge_file_with_client(&client, 42, 777).expect("file");

        assert_eq!(file.id, 777);
        assert_eq!(file.file_name, "addon.zip");
        assert_eq!(
            client.requests.borrow()[0].url,
            "https://api.curseforge.com/v1/mods/42/files/777"
        );
    }

    #[test]
    fn fetch_curseforge_file_with_client_rejects_invalid_file_contracts() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let client = SingleRouteHttpClient::new(
            "https://api.curseforge.com/v1/mods/42/files/777",
            r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"not-a-timestamp","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"releaseType":1}}"#,
        );

        let error =
            fetch_curseforge_file_with_client(&client, 42, 777).expect_err("invalid file metadata");

        assert!(error.to_string().contains("file date must be"));
    }

    #[test]
    fn search_curseforge_mods_with_client_returns_validated_payload() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let client = CurseForgeSearchHttpClient::new(
            r#"{"data":[{"id":42,"name":"WeakAuras","summary":"  Aura tracking  ","downloadCount":100,"latestFilesIndexes":[{"fileId":777,"gameVersionTypeId":517},{"fileId":778,"gameVersionTypeId":517}],"links":{"websiteUrl":"https://www.curseforge.com/wow/addons/weakauras-2"}},{"id":43,"name":"SharedMedia","summary":"   ","downloadCount":50,"latestFilesIndexes":[],"links":{"websiteUrl":null}}]}"#,
        );

        let (wow_context, mods) =
            search_curseforge_mods_with_client(&client, "weak", WowFlavor::Retail, 100)
                .expect("search");

        assert_eq!(wow_context.game_id, 1);
        assert_eq!(wow_context.version_type_id, 517);
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].id, 42);
        assert_eq!(mods[0].summary.as_deref(), Some("  Aura tracking  "));
        assert_eq!(mods[0].latest_files_indexes[0].file_id, 777);
        assert_eq!(mods[1].id, 43);

        let requests = client.requests.borrow();
        let search_request = requests
            .iter()
            .find(|request| request.url == "https://api.curseforge.com/v1/mods/search")
            .expect("search request");
        assert!(
            search_request
                .query
                .contains(&("pageSize".to_string(), "50".to_string()))
        );
        assert!(
            search_request
                .query
                .contains(&("searchFilter".to_string(), "weak".to_string()))
        );
        assert!(
            search_request
                .headers
                .iter()
                .any(|header| header.name == "x-api-key" && header.value == "test-api-key")
        );
    }

    #[test]
    fn search_curseforge_mods_with_client_rejects_invalid_result_contracts() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let cases = [
            (
                r#"{"data":[{"id":0,"name":"WeakAuras","summary":null,"downloadCount":100,"latestFilesIndexes":[],"links":{"websiteUrl":"https://example.com/weakauras"}}]}"#,
                "mod id must be greater than zero",
            ),
            (
                r#"{"data":[{"id":42,"name":" ","summary":null,"downloadCount":100,"latestFilesIndexes":[],"links":{"websiteUrl":"https://example.com/weakauras"}}]}"#,
                "name `42` must not be empty",
            ),
            (
                r#"{"data":[{"id":42,"name":" WeakAuras","summary":null,"downloadCount":100,"latestFilesIndexes":[],"links":{"websiteUrl":"https://example.com/weakauras"}}]}"#,
                "name `42` must not have surrounding whitespace",
            ),
            (
                r#"{"data":[{"id":42,"name":"WeakAuras","summary":null,"downloadCount":100,"latestFilesIndexes":[],"links":{"websiteUrl":"ftp://example.com/weakauras"}}]}"#,
                "website URL must start with",
            ),
            (
                r#"{"data":[{"id":42,"name":"WeakAuras","summary":null,"downloadCount":100,"latestFilesIndexes":[{"fileId":0,"gameVersionTypeId":517}],"links":{"websiteUrl":"https://example.com/weakauras"}}]}"#,
                "latest file index file id",
            ),
            (
                r#"{"data":[{"id":42,"name":"WeakAuras","summary":null,"downloadCount":100,"latestFilesIndexes":[{"fileId":777,"gameVersionTypeId":0}],"links":{"websiteUrl":"https://example.com/weakauras"}}]}"#,
                "latest file index game version type id",
            ),
        ];

        for (body, expected_message) in cases {
            let client = CurseForgeSearchHttpClient::new(body);
            let error = search_curseforge_mods_with_client(&client, "weak", WowFlavor::Retail, 10)
                .expect_err("invalid search result");

            assert!(
                error.to_string().contains(expected_message),
                "expected `{}` in `{}`",
                expected_message,
                error
            );
        }
    }

    #[test]
    fn search_curseforge_mods_with_client_rejects_invalid_game_context_contracts() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let cases = [
            (
                r#"{"data":[{"id":0,"name":"World of Warcraft","slug":"world-of-warcraft"}]}"#,
                "CurseForge game id must be greater than zero",
            ),
            (
                r#"{"data":[{"id":1,"name":" ","slug":"world-of-warcraft"}]}"#,
                "CurseForge game name `1` must not be empty",
            ),
            (
                r#"{"data":[{"id":1,"name":" World of Warcraft","slug":"world-of-warcraft"}]}"#,
                "CurseForge game name `1` must not have surrounding whitespace",
            ),
            (
                r#"{"data":[{"id":1,"name":"World of Warcraft","slug":"world-of-warcraft"},{"id":1,"name":"World of Warcraft Classic","slug":"wow-classic"}]}"#,
                "duplicate game id",
            ),
        ];

        for (games_body, expected_message) in cases {
            let client = CurseForgeSearchHttpClient::new(valid_curseforge_search_body())
                .with_games_body(games_body);
            let error = search_curseforge_mods_with_client(&client, "weak", WowFlavor::Retail, 10)
                .expect_err("invalid game context");

            assert!(
                error.to_string().contains(expected_message),
                "expected `{}` in `{}`",
                expected_message,
                error
            );
        }
    }

    #[test]
    fn search_curseforge_mods_with_client_rejects_invalid_version_type_contracts() {
        let _guard = curseforge_api_key_guard("test-api-key");
        let cases = [
            (
                r#"{"data":[{"id":0,"name":"WoW Retail","slug":"wow_retail"}]}"#,
                "CurseForge game version type id must be greater than zero",
            ),
            (
                r#"{"data":[{"id":517,"name":" ","slug":"wow_retail"}]}"#,
                "CurseForge game version type name `517` must not be empty",
            ),
            (
                r#"{"data":[{"id":517,"name":"WoW Retail","slug":" wow_retail"}]}"#,
                "CurseForge game version type slug `517` must not have surrounding whitespace",
            ),
            (
                r#"{"data":[{"id":517,"name":"WoW Retail","slug":"wow_retail"},{"id":517,"name":"Retail","slug":"retail"}]}"#,
                "duplicate version type id",
            ),
        ];

        for (version_types_body, expected_message) in cases {
            let client = CurseForgeSearchHttpClient::new(valid_curseforge_search_body())
                .with_version_types_body(version_types_body);
            let error = search_curseforge_mods_with_client(&client, "weak", WowFlavor::Retail, 10)
                .expect_err("invalid version type context");

            assert!(
                error.to_string().contains(expected_message),
                "expected `{}` in `{}`",
                expected_message,
                error
            );
        }
    }

    struct CurseForgeSearchHttpClient<'a> {
        games_body: &'a str,
        version_types_body: &'a str,
        search_body: &'a str,
        requests: RefCell<Vec<HttpRequest>>,
    }

    impl<'a> CurseForgeSearchHttpClient<'a> {
        fn new(search_body: &'a str) -> Self {
            Self {
                games_body: r#"{"data":[{"id":1,"name":"World of Warcraft","slug":"world-of-warcraft"}]}"#,
                version_types_body: r#"{"data":[{"id":517,"name":"WoW Retail","slug":"wow_retail"}]}"#,
                search_body,
                requests: RefCell::new(Vec::new()),
            }
        }

        fn with_games_body(mut self, games_body: &'a str) -> Self {
            self.games_body = games_body;
            self
        }

        fn with_version_types_body(mut self, version_types_body: &'a str) -> Self {
            self.version_types_body = version_types_body;
            self
        }
    }

    impl HttpClient for CurseForgeSearchHttpClient<'_> {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            self.requests.borrow_mut().push(request.clone());
            match request.url.as_str() {
                "https://api.curseforge.com/v1/games" => Ok(HttpResponse {
                    status_code: 200,
                    body: self.games_body.to_string(),
                }),
                "https://api.curseforge.com/v1/games/1/version-types" => Ok(HttpResponse {
                    status_code: 200,
                    body: self.version_types_body.to_string(),
                }),
                "https://api.curseforge.com/v1/mods/search" => Ok(HttpResponse {
                    status_code: 200,
                    body: self.search_body.to_string(),
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

    fn valid_curseforge_search_body() -> &'static str {
        r#"{"data":[{"id":42,"name":"WeakAuras","summary":"Aura tracking","downloadCount":100,"latestFilesIndexes":[{"fileId":777,"gameVersionTypeId":517}],"links":{"websiteUrl":"https://example.com/weakauras"}}]}"#
    }

    struct SingleRouteHttpClient<'a> {
        expected_url: &'a str,
        body: &'a str,
        requests: RefCell<Vec<HttpRequest>>,
    }

    impl<'a> SingleRouteHttpClient<'a> {
        fn new(expected_url: &'a str, body: &'a str) -> Self {
            Self {
                expected_url,
                body,
                requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for SingleRouteHttpClient<'_> {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            self.requests.borrow_mut().push(request.clone());
            if request.url == self.expected_url {
                return Ok(HttpResponse {
                    status_code: 200,
                    body: self.body.to_string(),
                });
            }

            Err(AppError::Validation(format!(
                "unexpected request url: {}",
                request.url
            )))
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
}
