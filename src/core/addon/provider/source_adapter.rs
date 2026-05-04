use super::curseforge::{
    CurseForgeFileReleaseType, required_dependency_mod_ids_for_curseforge_file,
    resolve_curseforge_file_with_client,
};
use super::http::HttpClient;
use super::source::source_kind_label;
use super::{
    AddonProviderContext, AddonSourceRef, AddonSourceResolutionPolicy, ResolvedAddonDependencies,
};
use crate::core::addon::policy::AddonReleaseChannel;
use crate::core::error::{AppError, AppResult};
pub(super) fn resolve_source_dependencies_impl(
    http_client: &impl HttpClient,
    source: &AddonSourceRef,
    context: AddonProviderContext<'_>,
) -> AppResult<ResolvedAddonDependencies> {
    match source {
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => {
            let file = resolve_curseforge_file_with_client(
                http_client,
                *mod_id,
                *file_id,
                context.target_flavor,
                curseforge_release_type_limit(context.resolution_policy),
            )?;
            Ok(ResolvedAddonDependencies::missing_required_only(
                curseforge_mod_id_dependencies_to_source_refs(
                    required_dependency_mod_ids_for_curseforge_file(*mod_id, &file.dependencies),
                ),
            ))
        }
        _ => Err(AppError::Validation(format!(
            "addon dependency installation is currently only supported for CurseForge sources, but `{}` uses `{}`",
            source.display_name(),
            source_kind_label(source),
        ))),
    }
}

fn curseforge_mod_id_dependencies_to_source_refs(mod_ids: Vec<u32>) -> Vec<AddonSourceRef> {
    mod_ids
        .into_iter()
        .map(|mod_id| AddonSourceRef::CurseForgeMod {
            mod_id,
            file_id: None,
        })
        .collect()
}

pub(super) fn github_allows_prerelease(policy: AddonSourceResolutionPolicy) -> bool {
    match policy.allow_prerelease {
        Some(value) => value,
        None => matches!(
            policy.release_channel,
            Some(AddonReleaseChannel::Beta | AddonReleaseChannel::Alpha)
        ),
    }
}

pub(super) fn curseforge_release_type_limit(
    policy: AddonSourceResolutionPolicy,
) -> Option<CurseForgeFileReleaseType> {
    if matches!(policy.allow_prerelease, Some(false)) {
        return Some(CurseForgeFileReleaseType::Stable);
    }

    match policy.release_channel {
        Some(AddonReleaseChannel::Stable) => Some(CurseForgeFileReleaseType::Stable),
        Some(AddonReleaseChannel::Beta) => Some(CurseForgeFileReleaseType::Beta),
        Some(AddonReleaseChannel::Alpha) => Some(CurseForgeFileReleaseType::Alpha),
        None if matches!(policy.allow_prerelease, Some(true)) => {
            Some(CurseForgeFileReleaseType::Alpha)
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::super::AddonDependencyResolutionStrategy;
    use super::super::http::{
        HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse, HttpRequest,
        HttpResponse,
    };
    use super::super::test_support::{curseforge_api_key_guard, standard_curseforge_api_key_guard};
    use super::*;
    use crate::core::task::CancellationToken;

    #[test]
    fn resolve_source_dependencies_returns_required_curseforge_dependencies() {
        #[derive(Default)]
        struct FakeHttpClient {
            requests: RefCell<Vec<HttpRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                self.requests.borrow_mut().push(request.clone());
                match request.url.as_str() {
                    "https://api.curseforge.com/v1/mods/42/files/777" => Ok(HttpResponse {
                        status_code: 200,
                        body: r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"releaseType":1,"dependencies":[{"modId":99,"relationType":3},{"modId":99,"relationType":3},{"modId":100,"relationType":2},{"modId":101,"relationType":9},{"modId":42,"relationType":3}]}}"#.to_string(),
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

        let _guard = curseforge_api_key_guard("test-api-key");
        let http_client = FakeHttpClient::default();
        let dependencies = resolve_source_dependencies_impl(
            &http_client,
            &AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: Some(777),
            },
            AddonProviderContext::default(),
        )
        .expect("resolve dependencies");

        assert_eq!(
            dependencies.strategy,
            AddonDependencyResolutionStrategy::MissingRequiredOnly
        );
        assert_eq!(
            dependencies.dependencies,
            vec![AddonSourceRef::CurseForgeMod {
                mod_id: 99,
                file_id: None,
            }]
        );
        assert_eq!(http_client.requests.borrow().len(), 1);
    }

    #[test]
    fn resolve_source_dependencies_accepts_standard_curseforge_api_key_env() {
        #[derive(Default)]
        struct FakeHttpClient {
            requests: RefCell<Vec<HttpRequest>>,
        }

        impl HttpClient for FakeHttpClient {
            fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
                assert!(
                    request
                        .headers
                        .iter()
                        .any(|header| header.name == "x-api-key" && header.value == "standard-key")
                );
                self.requests.borrow_mut().push(request.clone());
                Ok(HttpResponse {
                    status_code: 200,
                    body: r#"{"data":{"id":777,"fileName":"addon.zip","fileDate":"2026-04-21T12:00:00Z","downloadUrl":"https://example.com/curseforge/777/addon.zip","isAvailable":true,"releaseType":1}}"#.to_string(),
                })
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

        let _guard = standard_curseforge_api_key_guard("standard-key");
        let http_client = FakeHttpClient::default();
        let dependencies = resolve_source_dependencies_impl(
            &http_client,
            &AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: Some(777),
            },
            AddonProviderContext::default(),
        )
        .expect("resolve dependencies with standard env");

        assert!(dependencies.dependencies.is_empty());
        assert_eq!(http_client.requests.borrow().len(), 1);
    }
}
