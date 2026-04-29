use std::cell::{Cell, RefCell};

use tempfile::tempdir;

use super::super::http::{
    HttpClient, HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse,
    HttpRequest, HttpResponse,
};
use super::super::test_support::{curseforge_api_key_guard, successful_download_response};
use super::super::{AddonSourceRef, DefaultAddonProvider};
use super::materialize_source_twice;
use crate::core::error::{AppError, AppResult};
use crate::core::task::CancellationToken;

#[test]
fn default_addon_provider_redownloads_cached_curseforge_file_when_remote_validator_changes() {
    let _guard = curseforge_api_key_guard("test-api-key");

    struct FakeHttpClient {
        file_calls: Cell<usize>,
        downloads: RefCell<Vec<HttpDownloadRequest>>,
    }

    impl Default for FakeHttpClient {
        fn default() -> Self {
            Self {
                file_calls: Cell::new(0),
                downloads: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for FakeHttpClient {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            if !request.url.ends_with("/mods/42/files/777") {
                return Err(AppError::Validation(format!(
                    "unexpected request url: {}",
                    request.url
                )));
            }
            let next_call = self.file_calls.get() + 1;
            self.file_calls.set(next_call);
            let body = match next_call {
                1 => {
                    r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-20T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"fileLength":17,"hashes":[{"value":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","algo":1},{"value":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","algo":2}]}}"#
                }
                _ => {
                    r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"fileLength":21,"hashes":[{"value":"cccccccccccccccccccccccccccccccccccccccc","algo":1},{"value":"dddddddddddddddddddddddddddddddd","algo":2}]}}"#
                }
            };
            Ok(HttpResponse {
                status_code: 200,
                body: body.to_string(),
            })
        }

        fn download_to_path(
            &self,
            request: HttpDownloadRequest,
            _cancellation: &dyn CancellationToken,
            _observer: Option<&dyn HttpDownloadProgressObserver>,
        ) -> AppResult<HttpDownloadResponse> {
            self.downloads.borrow_mut().push(request.clone());
            let payload = format!("archive-download-{}", self.downloads.borrow().len());
            std::fs::write(&request.destination, payload).expect("archive file");
            Ok(successful_download_response(Vec::new()))
        }
    }

    let temp = tempdir().expect("temp dir");
    let provider = DefaultAddonProvider::with_http_client(FakeHttpClient::default())
        .with_download_cache_dir(Some(temp.path().join("cache")));
    let source = AddonSourceRef::CurseForgeMod {
        mod_id: 42,
        file_id: Some(777),
    };

    let (first, second) = materialize_source_twice(&provider, &source, temp.path());

    assert_eq!(first.archive_path, second.archive_path);
    assert_eq!(provider.http_client().file_calls.get(), 2);
    assert_eq!(provider.http_client().downloads.borrow().len(), 2);
    assert_eq!(
        std::fs::read_to_string(&second.archive_path).expect("refreshed archive"),
        "archive-download-2"
    );
}
