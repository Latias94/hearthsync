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
