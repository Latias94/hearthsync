use serde::{Deserialize, Deserializer};

use super::http::{HttpClient, HttpHeader, HttpRequest};
use super::materialize::ResolvedDownloadArtifact;
use super::source::validate_wago_source_identifier;
use super::validation::RemoteArchiveValidators;
use super::{AddonSourceRef, AddonSourceResolutionPolicy};
use crate::core::addon::policy::AddonReleaseChannel;
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::boundary_validation::{is_rfc3339_timestamp_shape, validate_http_url};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

const WAGO_ADDONS_BASE: &str = "https://addons.wago.io";
const WAGO_SOURCE_INPUT_MESSAGE: &str =
    "Wago source must look like `wago:<project-id>[@release-id]`";
const WAGO_MAX_RELEASE_PAGES: u32 = 50;
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

pub(super) fn parse_wago_source(source: &str) -> AppResult<Option<AddonSourceRef>> {
    let Some(spec) = source.strip_prefix("wago:") else {
        return Ok(None);
    };

    let (project_id, release_id) = match spec.rsplit_once('@') {
        Some((left, right)) if !right.is_empty() => (left, Some(right.to_string())),
        Some(_) => return Err(AppError::Validation(WAGO_SOURCE_INPUT_MESSAGE.to_string())),
        None => (spec, None),
    };
    validate_wago_source_identifier("Wago source input", "Wago project id", project_id)
        .map_err(|_| AppError::Validation(WAGO_SOURCE_INPUT_MESSAGE.to_string()))?;
    if let Some(release_id) = release_id.as_deref() {
        validate_wago_source_identifier("Wago source input", "Wago release id", release_id)
            .map_err(|_| AppError::Validation(WAGO_SOURCE_INPUT_MESSAGE.to_string()))?;
    }

    Ok(Some(AddonSourceRef::WagoAddon {
        project_id: project_id.to_string(),
        release_id,
    }))
}

pub(super) fn resolve_wago_artifact_with_client(
    client: &impl HttpClient,
    project_id: &str,
    release_id: Option<&str>,
    target_flavor: Option<WowFlavor>,
    policy: AddonSourceResolutionPolicy,
) -> AppResult<ResolvedDownloadArtifact> {
    validate_wago_source_identifier("Wago source", "Wago project id", project_id)?;
    if let Some(release_id) = release_id {
        validate_wago_source_identifier("Wago source", "Wago release id", release_id)?;
    }

    let release = match release_id {
        Some(release_id) => find_wago_release_by_id(client, project_id, release_id, target_flavor)?,
        None => select_latest_wago_release(client, project_id, target_flavor, policy)?,
    };
    let download_url = release.download_link.clone().ok_or_else(|| {
        AppError::Validation(format!(
            "Wago release `{}` does not provide a download URL",
            release.id
        ))
    })?;
    validate_http_url(
        &download_url,
        &format!("Wago release `{}` download URL", release.id),
    )?;

    Ok(ResolvedDownloadArtifact {
        cache_source_ref: AddonSourceRef::WagoAddon {
            project_id: project_id.to_string(),
            release_id: Some(release.id.clone()),
        },
        download_url,
        archive_name: wago_archive_name(project_id, &release.id)?,
        headers: wago_download_headers(),
        remote_validators: RemoteArchiveValidators::default(),
    })
}

fn select_latest_wago_release(
    client: &impl HttpClient,
    project_id: &str,
    target_flavor: Option<WowFlavor>,
    policy: AddonSourceResolutionPolicy,
) -> AppResult<WagoRelease> {
    let mut candidates = Vec::new();
    for stability in allowed_wago_stabilities(policy) {
        if let Some(release) =
            find_first_matching_release_for_stability(client, project_id, stability, target_flavor)?
        {
            candidates.push(release);
        }
    }

    candidates.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    candidates.into_iter().next().ok_or_else(|| {
        AppError::Validation(format!(
            "Wago addon `{project_id}` does not expose a downloadable release matching the requested policy and target flavor"
        ))
    })
}

fn find_first_matching_release_for_stability(
    client: &impl HttpClient,
    project_id: &str,
    stability: WagoReleaseStability,
    target_flavor: Option<WowFlavor>,
) -> AppResult<Option<WagoRelease>> {
    let mut page_number = 1;
    loop {
        let page = fetch_wago_release_page_with_client(client, project_id, stability, page_number)?;
        let last_page_to_scan = wago_last_page_to_scan(&page);
        if let Some(release) = page
            .data
            .into_iter()
            .find(|release| release_is_latest_candidate(release, stability, target_flavor))
        {
            validate_wago_download_release(&release)?;
            return Ok(Some(release));
        }

        if page_number >= last_page_to_scan {
            return Ok(None);
        }
        page_number += 1;
    }
}

fn find_wago_release_by_id(
    client: &impl HttpClient,
    project_id: &str,
    release_id: &str,
    target_flavor: Option<WowFlavor>,
) -> AppResult<WagoRelease> {
    for stability in [
        WagoReleaseStability::Stable,
        WagoReleaseStability::Beta,
        WagoReleaseStability::Alpha,
    ] {
        let mut page_number = 1;
        loop {
            let page =
                fetch_wago_release_page_with_client(client, project_id, stability, page_number)?;
            let last_page_to_scan = wago_last_page_to_scan(&page);
            if let Some(release) = page
                .data
                .into_iter()
                .find(|release| release.id == release_id)
            {
                validate_wago_download_release(&release)?;
                if !wago_release_matches_flavor(&release, target_flavor) {
                    return Err(AppError::Validation(format!(
                        "Wago release `{release_id}` does not match target flavor `{}`",
                        target_flavor
                            .map(|flavor| flavor.as_str())
                            .unwrap_or("unspecified")
                    )));
                }
                return Ok(release);
            }

            if page_number >= last_page_to_scan {
                break;
            }
            page_number += 1;
        }
    }

    Err(AppError::NotFound(format!(
        "Wago release `{release_id}` was not found for addon `{project_id}` in the first {WAGO_MAX_RELEASE_PAGES} pages of each stability channel"
    )))
}

fn fetch_wago_release_page_with_client(
    client: &impl HttpClient,
    project_id: &str,
    stability: WagoReleaseStability,
    page: u32,
) -> AppResult<WagoReleasePage> {
    let response = client.get(
        HttpRequest::new(format!("{WAGO_ADDONS_BASE}/addons/{project_id}/versions"))
            .with_headers(wago_page_headers())
            .with_query(vec![
                ("stability".to_string(), stability.as_str().to_string()),
                ("page".to_string(), page.to_string()),
            ]),
    )?;
    if !response.is_success() {
        return Err(AppError::Validation(format!(
            "Wago request failed with HTTP status {}",
            response.status_code
        )));
    }

    parse_wago_release_page_html(&response.body)
}

fn parse_wago_release_page_html(html: &str) -> AppResult<WagoReleasePage> {
    let json = extract_inertia_data_page_json(html)?;
    let page: WagoInertiaPage = serde_json::from_str(&json)?;
    Ok(page.props.releases)
}

fn extract_inertia_data_page_json(html: &str) -> AppResult<String> {
    let needle = "data-page=";
    let start = html.find(needle).ok_or_else(|| {
        AppError::Validation("Wago releases page did not include Inertia data-page".to_string())
    })? + needle.len();
    let Some(quote) = html[start..].chars().next() else {
        return Err(AppError::Validation(
            "Wago releases page had an empty Inertia data-page attribute".to_string(),
        ));
    };
    if !matches!(quote, '"' | '\'') {
        return Err(AppError::Validation(
            "Wago releases page had an invalid Inertia data-page attribute".to_string(),
        ));
    }
    let value_start = start + quote.len_utf8();
    let value_end = html[value_start..]
        .find(quote)
        .map(|index| value_start + index)
        .ok_or_else(|| {
            AppError::Validation(
                "Wago releases page had an unterminated Inertia data-page attribute".to_string(),
            )
        })?;

    Ok(decode_html_entities(&html[value_start..value_end]))
}

fn decode_html_entities(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        output.push_str(&rest[..index]);
        let after_ampersand = &rest[index + 1..];
        if let Some(end) = after_ampersand.find(';') {
            let entity = &after_ampersand[..end];
            if let Some(decoded) = decode_html_entity(entity) {
                output.push(decoded);
                rest = &after_ampersand[end + 1..];
                continue;
            }
        }

        output.push('&');
        rest = after_ampersand;
    }
    output.push_str(rest);
    output
}

fn decode_html_entity(entity: &str) -> Option<char> {
    match entity {
        "quot" => Some('"'),
        "apos" | "#039" => Some('\''),
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        _ => decode_numeric_html_entity(entity),
    }
}

fn decode_numeric_html_entity(entity: &str) -> Option<char> {
    let value = if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        u32::from_str_radix(hex, 16).ok()?
    } else if let Some(decimal) = entity.strip_prefix('#') {
        decimal.parse::<u32>().ok()?
    } else {
        return None;
    };

    char::from_u32(value)
}

fn release_is_latest_candidate(
    release: &WagoRelease,
    stability: WagoReleaseStability,
    target_flavor: Option<WowFlavor>,
) -> bool {
    release.is_processed
        && release.stability.eq_ignore_ascii_case(stability.as_str())
        && wago_release_matches_flavor(release, target_flavor)
        && release
            .download_link
            .as_deref()
            .is_some_and(|url| !url.trim().is_empty())
}

fn wago_release_matches_flavor(release: &WagoRelease, target_flavor: Option<WowFlavor>) -> bool {
    let Some(target_flavor) = target_flavor else {
        return true;
    };
    if !release.has_any_supported_patch() {
        return true;
    }

    match target_flavor {
        WowFlavor::Retail | WowFlavor::Ptr | WowFlavor::Beta | WowFlavor::Xptr => {
            !release.supported_retail_patches.is_empty()
        }
        WowFlavor::Classic => {
            !release.supported_mop_patches.is_empty()
                || !release.supported_cata_patches.is_empty()
                || !release.supported_wotlk_patches.is_empty()
                || !release.supported_bc_patches.is_empty()
                || !release.supported_classic_patches.is_empty()
        }
        WowFlavor::ClassicEra => !release.supported_classic_patches.is_empty(),
    }
}

fn validate_wago_download_release(release: &WagoRelease) -> AppResult<()> {
    validate_wago_source_identifier("Wago release", "Wago release id", &release.id)?;
    let Some(stability) = WagoReleaseStability::from_str(&release.stability) else {
        return Err(AppError::Validation(format!(
            "Wago release `{}` stability must be one of stable, beta, or alpha",
            release.id
        )));
    };
    if !release.stability.eq_ignore_ascii_case(stability.as_str()) {
        return Err(AppError::Validation(format!(
            "Wago release `{}` stability must not have surrounding whitespace",
            release.id
        )));
    }
    if !release.is_processed {
        return Err(AppError::Validation(format!(
            "Wago release `{}` is not processed yet",
            release.id
        )));
    }
    if release.size.is_some_and(|size| size == 0) {
        return Err(AppError::Validation(format!(
            "Wago release `{}` size must be greater than zero",
            release.id
        )));
    }
    if !is_rfc3339_timestamp_shape(&release.created_at) {
        return Err(AppError::Validation(format!(
            "Wago release `{}` created_at must be an RFC 3339 timestamp",
            release.id
        )));
    }
    let download_url = release.download_link.as_deref().ok_or_else(|| {
        AppError::Validation(format!(
            "Wago release `{}` does not provide a download URL",
            release.id
        ))
    })?;
    validate_http_url(
        download_url,
        &format!("Wago release `{}` download URL", release.id),
    )
}

fn allowed_wago_stabilities(policy: AddonSourceResolutionPolicy) -> Vec<WagoReleaseStability> {
    let max_stability = if matches!(policy.allow_prerelease, Some(false)) {
        WagoReleaseStability::Stable
    } else {
        match policy.release_channel {
            Some(AddonReleaseChannel::Stable) => WagoReleaseStability::Stable,
            Some(AddonReleaseChannel::Beta) => WagoReleaseStability::Beta,
            Some(AddonReleaseChannel::Alpha) => WagoReleaseStability::Alpha,
            None if matches!(policy.allow_prerelease, Some(true)) => WagoReleaseStability::Alpha,
            None => WagoReleaseStability::Stable,
        }
    };

    match max_stability {
        WagoReleaseStability::Stable => vec![WagoReleaseStability::Stable],
        WagoReleaseStability::Beta => {
            vec![WagoReleaseStability::Beta, WagoReleaseStability::Stable]
        }
        WagoReleaseStability::Alpha => vec![
            WagoReleaseStability::Alpha,
            WagoReleaseStability::Beta,
            WagoReleaseStability::Stable,
        ],
    }
}

fn wago_last_page_to_scan(page: &WagoReleasePage) -> u32 {
    page.last_page
        .max(page.current_page)
        .clamp(1, WAGO_MAX_RELEASE_PAGES)
}

fn wago_archive_name(project_id: &str, release_id: &str) -> AppResult<String> {
    let archive_name = format!("wago-{project_id}-{release_id}.zip");
    validate_portable_path_segment(&archive_name, "Wago archive")?;
    Ok(archive_name)
}

fn wago_page_headers() -> Vec<HttpHeader> {
    let mut headers = wago_download_headers();
    headers.push(HttpHeader {
        name: "Accept".to_string(),
        value: "text/html,application/xhtml+xml".to_string(),
    });
    headers
}

fn wago_download_headers() -> Vec<HttpHeader> {
    vec![HttpHeader {
        name: "User-Agent".to_string(),
        value: USER_AGENT_VALUE.to_string(),
    }]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WagoReleaseStability {
    Stable,
    Beta,
    Alpha,
}

impl WagoReleaseStability {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Alpha => "alpha",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "stable" => Some(Self::Stable),
            "beta" => Some(Self::Beta),
            "alpha" => Some(Self::Alpha),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct WagoInertiaPage {
    props: WagoReleaseProps,
}

#[derive(Debug, Clone, Deserialize)]
struct WagoReleaseProps {
    releases: WagoReleasePage,
}

#[derive(Debug, Clone, Deserialize)]
struct WagoReleasePage {
    #[serde(default)]
    current_page: u32,
    #[serde(default)]
    last_page: u32,
    #[serde(default)]
    data: Vec<WagoRelease>,
}

#[derive(Debug, Clone, Deserialize)]
struct WagoRelease {
    id: String,
    #[serde(default)]
    size: Option<u64>,
    stability: String,
    created_at: String,
    #[serde(default)]
    is_processed: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    supported_retail_patches: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    supported_cata_patches: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    supported_wotlk_patches: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    supported_bc_patches: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    supported_classic_patches: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    supported_mop_patches: Vec<String>,
    #[serde(default)]
    download_link: Option<String>,
}

impl WagoRelease {
    fn has_any_supported_patch(&self) -> bool {
        !self.supported_retail_patches.is_empty()
            || !self.supported_cata_patches.is_empty()
            || !self.supported_wotlk_patches.is_empty()
            || !self.supported_bc_patches.is_empty()
            || !self.supported_classic_patches.is_empty()
            || !self.supported_mop_patches.is_empty()
    }
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
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
    fn parse_wago_source_accepts_project_and_release_ids() {
        let source = parse_wago_source("wago:qv63A7Gb@vdx1042w")
            .expect("parse")
            .expect("source ref");

        assert_eq!(
            source,
            AddonSourceRef::WagoAddon {
                project_id: "qv63A7Gb".to_string(),
                release_id: Some("vdx1042w".to_string()),
            }
        );
        assert_eq!(source.display_name(), "wago:qv63A7Gb@vdx1042w");
    }

    #[test]
    fn parse_wago_source_rejects_empty_or_unsafe_ids() {
        for source in ["wago:", "wago:qv63A7Gb@", "wago:bad/id", "wago:bad id"] {
            let error = parse_wago_source(source).expect_err("invalid wago source");

            assert!(matches!(error, AppError::Validation(_)));
            assert!(error.to_string().contains("Wago source must look like"));
        }
    }

    #[test]
    fn parse_wago_release_page_extracts_inertia_payload() {
        let page = parse_wago_release_page_html(&wago_release_page_html(&[wago_release_json(
            "vdx1042w",
            "stable",
            "2026-05-01T16:42:38.000000Z",
            true,
        )]))
        .expect("release page");

        assert_eq!(page.data.len(), 1);
        assert_eq!(page.data[0].id, "vdx1042w");
        assert_eq!(
            page.data[0].download_link.as_deref(),
            Some("https://addons.wago.io/download/vdx1042w?x=1&y=2")
        );
    }

    #[test]
    fn parse_wago_release_page_accepts_null_supported_patch_arrays() {
        let release = r#"{"id":"retail1","size":1024,"label":"retail1","stability":"stable","created_at":"2026-05-01T00:00:00Z","is_processed":true,"supported_retail_patches":["12.0.5"],"supported_mop_patches":null,"supported_cata_patches":null,"supported_wotlk_patches":null,"supported_bc_patches":null,"supported_classic_patches":null,"download_link":"https://addons.wago.io/download/retail1"}"#.to_string();
        let page = parse_wago_release_page_html(&wago_release_page_html(&[release]))
            .expect("release page");

        assert_eq!(page.data.len(), 1);
        assert_eq!(page.data[0].supported_retail_patches, vec!["12.0.5"]);
        assert!(page.data[0].supported_mop_patches.is_empty());
        assert!(page.data[0].supported_cata_patches.is_empty());
        assert!(page.data[0].supported_wotlk_patches.is_empty());
        assert!(page.data[0].supported_bc_patches.is_empty());
        assert!(page.data[0].supported_classic_patches.is_empty());
        assert!(wago_release_matches_flavor(
            &page.data[0],
            Some(WowFlavor::Retail)
        ));
        assert!(!wago_release_matches_flavor(
            &page.data[0],
            Some(WowFlavor::Classic)
        ));
    }

    #[test]
    fn resolve_wago_artifact_selects_latest_stable_release() {
        let client = WagoPageHttpClient::new(vec![wago_release_page_html(&[
            wago_release_json("new123", "stable", "2026-05-01T00:00:00Z", true),
            wago_release_json("old123", "stable", "2026-04-01T00:00:00Z", true),
        ])]);

        let artifact = resolve_wago_artifact_with_client(
            &client,
            "qv63A7Gb",
            None,
            Some(WowFlavor::Retail),
            AddonSourceResolutionPolicy::default(),
        )
        .expect("artifact");

        assert_eq!(
            artifact.download_url,
            "https://addons.wago.io/download/new123?x=1&y=2"
        );
        assert_eq!(artifact.archive_name, "wago-qv63A7Gb-new123.zip");
        assert_eq!(
            artifact.cache_source_ref,
            AddonSourceRef::WagoAddon {
                project_id: "qv63A7Gb".to_string(),
                release_id: Some("new123".to_string()),
            }
        );
        let requests = client.requests.borrow();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].url,
            "https://addons.wago.io/addons/qv63A7Gb/versions"
        );
        assert_eq!(
            requests[0].query,
            vec![
                ("stability".to_string(), "stable".to_string()),
                ("page".to_string(), "1".to_string())
            ]
        );
    }

    #[test]
    fn resolve_wago_artifact_searches_all_stabilities_for_exact_release() {
        let client = WagoPageHttpClient::new(vec![
            wago_release_page_html(&[]),
            wago_release_page_html(&[]),
            wago_release_page_html(&[wago_release_json(
                "alpha123",
                "alpha",
                "2026-05-01T00:00:00Z",
                true,
            )]),
        ]);

        let artifact = resolve_wago_artifact_with_client(
            &client,
            "qv63A7Gb",
            Some("alpha123"),
            Some(WowFlavor::Retail),
            AddonSourceResolutionPolicy::default(),
        )
        .expect("artifact");

        assert_eq!(
            artifact.download_url,
            "https://addons.wago.io/download/alpha123?x=1&y=2"
        );
        assert_eq!(client.requests.borrow().len(), 3);
    }

    #[test]
    fn resolve_wago_artifact_rejects_invalid_download_url_contract() {
        let invalid_release = r#"{"id":"badurl1","size":1024,"label":"badurl1","stability":"stable","created_at":"2026-05-01T00:00:00Z","is_processed":true,"supported_retail_patches":["12.0.5"],"download_link":"ftp://addons.wago.io/download/badurl1"}"#.to_string();
        let client = WagoPageHttpClient::new(vec![wago_release_page_html(&[invalid_release])]);

        let error = resolve_wago_artifact_with_client(
            &client,
            "qv63A7Gb",
            None,
            Some(WowFlavor::Retail),
            AddonSourceResolutionPolicy::default(),
        )
        .expect_err("invalid Wago download URL should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(
            error
                .to_string()
                .contains("download URL must start with `http://` or `https://`")
        );
    }

    fn wago_release_page_html(releases: &[String]) -> String {
        let json = format!(
            r#"{{"component":"Addon/Releases","props":{{"releases":{{"current_page":1,"last_page":1,"data":[{}]}}}}}}"#,
            releases.join(",")
        );
        format!(
            r#"<html><body><div id="app" data-page="{}"></div></body></html>"#,
            encode_data_page_attribute(&json)
        )
    }

    fn wago_release_json(
        id: &str,
        stability: &str,
        created_at: &str,
        supports_retail: bool,
    ) -> String {
        let retail_patches = if supports_retail { r#""12.0.5""# } else { "" };
        format!(
            r#"{{"id":"{id}","size":1024,"label":"{id}","stability":"{stability}","created_at":"{created_at}","is_processed":true,"supported_retail_patches":[{retail_patches}],"download_link":"https://addons.wago.io/download/{id}?x=1&y=2"}}"#
        )
    }

    fn encode_data_page_attribute(value: &str) -> String {
        value
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('\'', "&#039;")
    }

    struct WagoPageHttpClient {
        responses: RefCell<Vec<String>>,
        requests: RefCell<Vec<HttpRequest>>,
    }

    impl WagoPageHttpClient {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses: RefCell::new(responses),
                requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for WagoPageHttpClient {
        fn get(&self, request: HttpRequest) -> AppResult<HttpResponse> {
            self.requests.borrow_mut().push(request);
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
