use serde::Deserialize;

use super::http::{HttpClient, HttpRequest};
use super::materialize::ResolvedDownloadArtifact;
use super::source::{validate_tukui_slug, validate_tukui_version};
use super::validation::RemoteArchiveValidators;
use super::{AddonSearchResult, AddonSourceRef};
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::boundary_validation::validate_http_url;
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

const TUKUI_API_BASE: &str = "https://api.tukui.org/v1";
const TUKUI_SOURCE_INPUT_MESSAGE: &str =
    "Tukui source must look like `tukui:<slug>[@current-version]`";

pub(super) fn parse_tukui_source(source: &str) -> AppResult<Option<AddonSourceRef>> {
    let Some(spec) = source.strip_prefix("tukui:") else {
        return Ok(None);
    };

    let (slug, version) = match spec.rsplit_once('@') {
        Some((left, right)) if !right.is_empty() => (left, Some(right.to_string())),
        Some(_) => return Err(AppError::Validation(TUKUI_SOURCE_INPUT_MESSAGE.to_string())),
        None => (spec, None),
    };
    validate_tukui_slug("Tukui source input", "Tukui addon slug", slug)
        .map_err(|_| AppError::Validation(TUKUI_SOURCE_INPUT_MESSAGE.to_string()))?;
    if let Some(version) = version.as_deref() {
        validate_tukui_version("Tukui source input", "Tukui addon version", version)
            .map_err(|_| AppError::Validation(TUKUI_SOURCE_INPUT_MESSAGE.to_string()))?;
    }

    Ok(Some(AddonSourceRef::TukuiAddon {
        slug: slug.to_string(),
        version,
    }))
}

pub(super) fn resolve_tukui_artifact_with_client(
    client: &impl HttpClient,
    slug: &str,
    version: Option<&str>,
    target_flavor: Option<WowFlavor>,
) -> AppResult<ResolvedDownloadArtifact> {
    validate_tukui_slug("Tukui source", "Tukui addon slug", slug)?;
    if let Some(version) = version {
        validate_tukui_version("Tukui source", "Tukui addon version", version)?;
    }

    let addon = fetch_tukui_addon_with_client(client, slug)?;
    validate_tukui_addon(&addon, slug)?;
    if !tukui_addon_matches_flavor(&addon, target_flavor) {
        return Err(AppError::Validation(format!(
            "Tukui addon `{slug}` does not match target flavor `{}`",
            target_flavor
                .map(|flavor| flavor.as_str())
                .unwrap_or("unspecified")
        )));
    }
    if let Some(expected_version) = version
        && addon.version != expected_version
    {
        return Err(AppError::NotFound(format!(
            "Tukui addon `{slug}` current version `{}` does not match requested version `{expected_version}`; historical Tukui downloads are not supported",
            addon.version
        )));
    }

    Ok(ResolvedDownloadArtifact {
        cache_source_ref: AddonSourceRef::TukuiAddon {
            slug: addon.slug.clone(),
            version: Some(addon.version.clone()),
        },
        download_url: addon.url.clone(),
        archive_name: tukui_archive_name(&addon.slug, &addon.version)?,
        headers: Vec::new(),
        remote_validators: RemoteArchiveValidators::default(),
    })
}

pub(super) fn search_tukui_addons_with_client(
    client: &impl HttpClient,
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<Vec<AddonSearchResult>> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let mut addons = fetch_tukui_addon_list_with_client(client)?;
    addons.retain(|addon| {
        validate_tukui_addon(addon, &addon.slug).is_ok()
            && tukui_addon_matches_flavor(addon, Some(flavor))
            && tukui_addon_matches_query(addon, &query)
    });
    addons.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.slug.cmp(&right.slug))
    });

    Ok(addons
        .into_iter()
        .take(limit)
        .map(|addon| AddonSearchResult {
            provider: "Tukui",
            name: addon.name,
            summary: addon.small_desc,
            source: AddonSourceRef::TukuiAddon {
                slug: addon.slug.clone(),
                version: None,
            },
            install_hint: format!("tukui:{}", addon.slug),
            website_url: addon.web_url.or(addon.git_url),
            provider_project_id: None,
            provider_file_id: None,
            download_count: 0,
        })
        .collect())
}

fn fetch_tukui_addon_with_client(
    client: &impl HttpClient,
    slug: &str,
) -> AppResult<TukuiAddonPayload> {
    let response = client.get(HttpRequest::new(format!("{TUKUI_API_BASE}/addon/{slug}")))?;
    if !response.is_success() {
        return Err(AppError::Validation(format!(
            "Tukui request failed with HTTP status {}",
            response.status_code
        )));
    }

    Ok(serde_json::from_str(&response.body)?)
}

fn fetch_tukui_addon_list_with_client(
    client: &impl HttpClient,
) -> AppResult<Vec<TukuiAddonPayload>> {
    let response = client.get(HttpRequest::new(format!("{TUKUI_API_BASE}/addons")))?;
    if !response.is_success() {
        return Err(AppError::Validation(format!(
            "Tukui catalog request failed with HTTP status {}",
            response.status_code
        )));
    }

    Ok(serde_json::from_str(&response.body)?)
}

fn validate_tukui_addon(addon: &TukuiAddonPayload, expected_slug: &str) -> AppResult<()> {
    validate_tukui_slug("Tukui addon response", "Tukui addon slug", &addon.slug)?;
    if !addon.slug.eq_ignore_ascii_case(expected_slug) {
        return Err(AppError::Validation(format!(
            "Tukui addon response slug `{}` did not match requested slug `{expected_slug}`",
            addon.slug
        )));
    }
    validate_tukui_version(
        "Tukui addon response",
        "Tukui addon version",
        &addon.version,
    )?;
    if addon.name.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "Tukui addon `{}` name must not be empty",
            addon.slug
        )));
    }
    validate_http_url(
        &addon.url,
        &format!("Tukui addon `{}` download URL", addon.slug),
    )?;
    if let Some(web_url) = addon.web_url.as_deref() {
        validate_http_url(
            web_url,
            &format!("Tukui addon `{}` website URL", addon.slug),
        )?;
    }
    if let Some(git_url) = addon.git_url.as_deref() {
        validate_http_url(git_url, &format!("Tukui addon `{}` git URL", addon.slug))?;
    }

    Ok(())
}

fn tukui_addon_matches_query(addon: &TukuiAddonPayload, query: &str) -> bool {
    addon.slug.to_ascii_lowercase().contains(query)
        || addon.name.to_ascii_lowercase().contains(query)
        || addon
            .small_desc
            .as_deref()
            .is_some_and(|summary| summary.to_ascii_lowercase().contains(query))
}

fn tukui_addon_matches_flavor(addon: &TukuiAddonPayload, target_flavor: Option<WowFlavor>) -> bool {
    let Some(target_flavor) = target_flavor else {
        return true;
    };
    if addon.patch.is_empty() {
        return true;
    }

    let majors = addon
        .patch
        .iter()
        .filter_map(|patch| patch.split('.').next()?.parse::<u32>().ok())
        .collect::<Vec<_>>();
    if majors.is_empty() {
        return true;
    }

    match target_flavor {
        WowFlavor::Retail | WowFlavor::Ptr | WowFlavor::Beta | WowFlavor::Xptr => {
            majors.iter().any(|major| *major >= 6)
        }
        WowFlavor::Classic => majors.iter().any(|major| (2..=5).contains(major)),
        WowFlavor::ClassicEra => majors.contains(&1),
    }
}

fn tukui_archive_name(slug: &str, version: &str) -> AppResult<String> {
    let archive_name = format!("tukui-{slug}-{version}.zip");
    validate_portable_path_segment(&archive_name, "Tukui archive")?;
    Ok(archive_name)
}

#[derive(Debug, Clone, Deserialize)]
struct TukuiAddonPayload {
    slug: String,
    name: String,
    url: String,
    version: String,
    #[serde(default)]
    patch: Vec<String>,
    #[serde(default)]
    web_url: Option<String>,
    #[serde(default)]
    git_url: Option<String>,
    #[serde(default)]
    small_desc: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::super::http::{
        HttpDownloadProgressObserver, HttpDownloadRequest, HttpDownloadResponse, HttpResponse,
    };
    use super::*;
    use crate::core::task::CancellationToken;

    #[test]
    fn parse_tukui_source_accepts_slug_and_current_version_guard() {
        let source = parse_tukui_source("tukui:elvui@15.13")
            .expect("parse")
            .expect("source ref");

        assert_eq!(
            source,
            AddonSourceRef::TukuiAddon {
                slug: "elvui".to_string(),
                version: Some("15.13".to_string()),
            }
        );
        assert_eq!(source.display_name(), "tukui:elvui@15.13");
    }

    #[test]
    fn parse_tukui_source_rejects_empty_or_unsafe_ids() {
        for source in ["tukui:", "tukui:elvui@", "tukui:bad/id", "tukui:bad id"] {
            let error = parse_tukui_source(source).expect_err("invalid tukui source");

            assert!(matches!(error, AppError::Validation(_)));
            assert!(error.to_string().contains("Tukui source must look like"));
        }
    }

    #[test]
    fn resolve_tukui_artifact_uses_current_version_for_cache_identity() {
        let client = TukuiHttpClient::new(vec![tukui_addon_json("elvui", "15.13")]);

        let artifact =
            resolve_tukui_artifact_with_client(&client, "elvui", None, Some(WowFlavor::Retail))
                .expect("artifact");

        assert_eq!(
            artifact.download_url,
            "https://api.tukui.org/v1/download/elvui/token"
        );
        assert_eq!(artifact.archive_name, "tukui-elvui-15.13.zip");
        assert_eq!(
            artifact.cache_source_ref,
            AddonSourceRef::TukuiAddon {
                slug: "elvui".to_string(),
                version: Some("15.13".to_string()),
            }
        );
        assert_eq!(
            client.requests.borrow().as_slice(),
            &["https://api.tukui.org/v1/addon/elvui".to_string()]
        );
    }

    #[test]
    fn resolve_tukui_artifact_rejects_mismatched_version_guard() {
        let client = TukuiHttpClient::new(vec![tukui_addon_json("elvui", "15.13")]);

        let error = resolve_tukui_artifact_with_client(&client, "elvui", Some("15.12"), None)
            .expect_err("mismatched current version should fail");

        assert!(matches!(error, AppError::NotFound(_)));
        assert!(
            error
                .to_string()
                .contains("historical Tukui downloads are not supported")
        );
    }

    #[test]
    fn search_tukui_addons_filters_catalog_by_query_and_flavor() {
        let client = TukuiHttpClient::new(vec![format!(
            "[{},{}]",
            tukui_addon_json("elvui", "15.13"),
            tukui_addon_json_with_patches("classic-only", "1.0.0", &["1.15.8"])
        )]);

        let results =
            search_tukui_addons_with_client(&client, "elv", WowFlavor::Retail, 10).expect("search");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "Tukui");
        assert_eq!(results[0].name, "ElvUI");
        assert_eq!(results[0].install_hint, "tukui:elvui");
        assert_eq!(
            results[0].source,
            AddonSourceRef::TukuiAddon {
                slug: "elvui".to_string(),
                version: None,
            }
        );
    }

    fn tukui_addon_json(slug: &str, version: &str) -> String {
        tukui_addon_json_with_patches(slug, version, &["12.0.1", "5.5.3", "1.15.8"])
    }

    fn tukui_addon_json_with_patches(slug: &str, version: &str, patches: &[&str]) -> String {
        let name = if slug == "elvui" { "ElvUI" } else { slug };
        let patch_json = patches
            .iter()
            .map(|patch| format!(r#""{patch}""#))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            r#"{{"slug":"{slug}","name":"{name}","url":"https://api.tukui.org/v1/download/{slug}/token","version":"{version}","patch":[{patch_json}],"web_url":"https://tukui.org/{slug}","git_url":"https://github.com/tukui-org/{name}","small_desc":"A UI package"}}"#
        )
    }

    struct TukuiHttpClient {
        responses: RefCell<Vec<String>>,
        requests: RefCell<Vec<String>>,
    }

    impl TukuiHttpClient {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: RefCell::new(responses),
                requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for TukuiHttpClient {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            self.requests.borrow_mut().push(request.url);
            let response = self.responses.borrow_mut().remove(0);
            Ok(HttpResponse {
                status_code: 200,
                body: response,
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
}
