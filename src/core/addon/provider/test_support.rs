use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use super::cache::{CachedArchiveMetadata, cached_archive_metadata_path};
use super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpHeader, HttpRequest, HttpResponse,
};
use crate::core::error::AppResult;
use crate::core::task::CancellationToken;

pub(super) struct NoopHttpClient;

impl HttpClient for NoopHttpClient {
    fn get(&self, _request: HttpRequest) -> AppResult<HttpResponse> {
        panic!("get should not be called in this test")
    }

    fn download_to_path(
        &self,
        _request: HttpDownloadRequest,
        _cancellation: &dyn CancellationToken,
        _observer: Option<&dyn HttpDownloadProgressObserver>,
    ) -> AppResult<HttpDownloadResponse> {
        panic!("download should not be called in this test")
    }
}

pub(super) fn cached_metadata_path(archive_path: &Path) -> PathBuf {
    cached_archive_metadata_path(archive_path)
}

pub(super) fn load_cached_archive_metadata_fixture(archive_path: &Path) -> CachedArchiveMetadata {
    let metadata_path = cached_archive_metadata_path(archive_path);
    let metadata_bytes = std::fs::read(metadata_path).expect("cache metadata bytes");
    serde_json::from_slice(&metadata_bytes).expect("cache metadata")
}

pub(super) fn write_cached_archive_metadata_fixture(
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
) {
    let metadata_path = cached_archive_metadata_path(archive_path);
    let metadata_bytes = serde_json::to_vec_pretty(metadata).expect("cache metadata json");
    std::fs::write(metadata_path, metadata_bytes).expect("cache metadata write");
}

pub(super) fn successful_download_response(headers: Vec<HttpHeader>) -> HttpDownloadResponse {
    HttpDownloadResponse {
        status_code: 200,
        headers,
    }
}

pub(super) fn not_modified_download_response(headers: Vec<HttpHeader>) -> HttpDownloadResponse {
    HttpDownloadResponse {
        status_code: 304,
        headers,
    }
}

pub(super) fn curseforge_api_key_guard(value: &str) -> CurseForgeApiKeyGuard {
    curseforge_api_key_env_guard(Some(value), None)
}

pub(super) fn standard_curseforge_api_key_guard(value: &str) -> CurseForgeApiKeyGuard {
    curseforge_api_key_env_guard(None, Some(value))
}

pub(super) fn github_token_guard(value: &str) -> GitHubTokenGuard {
    github_token_env_guard(Some(value), None)
}

pub(super) fn standard_github_token_guard(value: &str) -> GitHubTokenGuard {
    github_token_env_guard(None, Some(value))
}

fn curseforge_api_key_env_guard(
    hearthsync_value: Option<&str>,
    standard_value: Option<&str>,
) -> CurseForgeApiKeyGuard {
    static CURSEFORGE_API_KEY_ENV_MUTEX: Mutex<()> = Mutex::new(());
    let lock = CURSEFORGE_API_KEY_ENV_MUTEX
        .lock()
        .expect("curseforge api key env lock");
    let hearthsync_key = "HEARTHSYNC_CURSEFORGE_API_KEY";
    let standard_key = "CURSEFORGE_API_KEY";
    let previous_hearthsync = std::env::var_os(hearthsync_key);
    let previous_standard = std::env::var_os(standard_key);
    set_optional_env_var(hearthsync_key, hearthsync_value);
    set_optional_env_var(standard_key, standard_value);

    CurseForgeApiKeyGuard {
        hearthsync_key,
        previous_hearthsync,
        standard_key,
        previous_standard,
        _lock: lock,
    }
}

fn github_token_env_guard(
    hearthsync_value: Option<&str>,
    standard_value: Option<&str>,
) -> GitHubTokenGuard {
    static GITHUB_TOKEN_ENV_MUTEX: Mutex<()> = Mutex::new(());
    let lock = GITHUB_TOKEN_ENV_MUTEX
        .lock()
        .expect("github token env lock");
    let hearthsync_key = "HEARTHSYNC_GITHUB_TOKEN";
    let standard_key = "GITHUB_TOKEN";
    let previous_hearthsync = std::env::var_os(hearthsync_key);
    let previous_standard = std::env::var_os(standard_key);
    set_optional_env_var(hearthsync_key, hearthsync_value);
    set_optional_env_var(standard_key, standard_value);

    GitHubTokenGuard {
        hearthsync_key,
        previous_hearthsync,
        standard_key,
        previous_standard,
        _lock: lock,
    }
}

pub(super) struct CurseForgeApiKeyGuard {
    hearthsync_key: &'static str,
    previous_hearthsync: Option<OsString>,
    standard_key: &'static str,
    previous_standard: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl Drop for CurseForgeApiKeyGuard {
    fn drop(&mut self) {
        restore_env_var(self.hearthsync_key, &self.previous_hearthsync);
        restore_env_var(self.standard_key, &self.previous_standard);
    }
}

pub(super) struct GitHubTokenGuard {
    hearthsync_key: &'static str,
    previous_hearthsync: Option<OsString>,
    standard_key: &'static str,
    previous_standard: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl Drop for GitHubTokenGuard {
    fn drop(&mut self) {
        restore_env_var(self.hearthsync_key, &self.previous_hearthsync);
        restore_env_var(self.standard_key, &self.previous_standard);
    }
}

fn set_optional_env_var(key: &str, value: Option<&str>) {
    match value {
        Some(value) => unsafe {
            std::env::set_var(key, value);
        },
        None => unsafe {
            std::env::remove_var(key);
        },
    }
}

fn restore_env_var(key: &str, value: &Option<OsString>) {
    match value {
        Some(value) => unsafe {
            std::env::set_var(key, value);
        },
        None => unsafe {
            std::env::remove_var(key);
        },
    }
}
