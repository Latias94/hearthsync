use std::path::PathBuf;
use std::sync::OnceLock;

use super::governance::{load_addon_index_governance, parse_addon_index_governance};
use super::storage::{inspect_addon_index, parse_addon_index};
use super::{AddonIndexPackage, AddonIndexSearch, AddonIndexSearchRequest};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

const COMMUNITY_ADDON_INDEX_LABEL: &str = "builtin:community-addon-index";
const COMMUNITY_ADDON_INDEX_TOML: &str =
    include_str!("../../../../catalog/community-addon-index.toml");
const COMMUNITY_ADDON_INDEX_GOVERNANCE_JSON: &str =
    include_str!("../../../../catalog/community-addon-index.governance.json");

#[derive(Debug)]
struct CommunityAddonIndexCache {
    index_name: String,
    packages: Vec<CommunitySearchDoc>,
}

#[derive(Debug)]
struct CommunitySearchDoc {
    package: AddonIndexPackage,
    search_fields: Vec<String>,
    normalized_id: String,
    normalized_supported_flavors: Vec<String>,
}

impl CommunitySearchDoc {
    fn new(
        package: AddonIndexPackage,
        governance: &super::governance::AddonIndexGovernance,
    ) -> Self {
        let normalized_id = package.id.to_ascii_lowercase();
        let normalized_supported_flavors = package
            .supported_flavors
            .iter()
            .map(|value| normalize_flavor_alias(value))
            .collect::<Vec<_>>();
        let search_fields = build_searchable_fields(
            &package,
            governance.searchable_terms_for_package(&package.id),
        );

        Self {
            package,
            search_fields,
            normalized_id,
            normalized_supported_flavors,
        }
    }

    fn supports_normalized_flavor(&self, normalized_flavor: &str) -> bool {
        self.normalized_supported_flavors.is_empty()
            || self
                .normalized_supported_flavors
                .iter()
                .any(|item| item == normalized_flavor)
    }

    fn search_score(&self, query: &str, query_tokens: &[String]) -> Option<u8> {
        search_score_from_fields(&self.search_fields, query, query_tokens)
    }
}

pub fn search_addon_index(request: AddonIndexSearchRequest) -> AppResult<AddonIndexSearch> {
    let inspection = inspect_addon_index(&request.index_path)?;
    let governance = load_addon_index_governance(&request.index_path)?;
    search_index_packages(
        inspection.index_path,
        inspection.index.name,
        request.query,
        request.limit,
        inspection.index.packages,
        governance.as_ref(),
    )
}

pub fn search_community_addon_index(
    query: String,
    limit: usize,
    flavor: WowFlavor,
) -> AppResult<AddonIndexSearch> {
    let cache = community_addon_index_cache()?;
    let normalized_flavor = normalize_flavor_alias(flavor.as_str());
    let packages = cache
        .packages
        .iter()
        .filter(|package| package.supports_normalized_flavor(&normalized_flavor))
        .collect::<Vec<_>>();
    search_community_search_docs(
        PathBuf::from(COMMUNITY_ADDON_INDEX_LABEL),
        cache.index_name.clone(),
        query,
        limit,
        packages,
    )
}

fn community_addon_index_cache() -> AppResult<&'static CommunityAddonIndexCache> {
    static COMMUNITY_ADDON_INDEX_CACHE: OnceLock<Result<CommunityAddonIndexCache, String>> =
        OnceLock::new();

    match COMMUNITY_ADDON_INDEX_CACHE.get_or_init(|| {
        parse_addon_index(COMMUNITY_ADDON_INDEX_TOML)
            .and_then(|index| {
                parse_addon_index_governance(COMMUNITY_ADDON_INDEX_GOVERNANCE_JSON).map(
                    |governance| CommunityAddonIndexCache {
                        index_name: index.name,
                        packages: index
                            .packages
                            .into_iter()
                            .map(|package| CommunitySearchDoc::new(package, &governance))
                            .collect::<Vec<_>>(),
                    },
                )
            })
            .map_err(|error| error.to_string())
    }) {
        Ok(cache) => Ok(cache),
        Err(error) => Err(AppError::Validation(format!(
            "failed to initialize builtin community addon index cache: {error}"
        ))),
    }
}

fn search_index_packages(
    index_path: PathBuf,
    index_name: String,
    query: String,
    limit: usize,
    packages: Vec<AddonIndexPackage>,
    governance: Option<&super::governance::AddonIndexGovernance>,
) -> AppResult<AddonIndexSearch> {
    let package_count = packages.len();
    let query = query.trim().to_string();
    let query_norm = normalize(&query);
    let query_tokens = tokenize_query(&query);
    let mut matches = packages
        .into_iter()
        .enumerate()
        .filter_map(|(ordinal, package)| {
            search_score(&package, governance, &query_norm, &query_tokens)
                .map(|score| (score, ordinal, package))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| {
                left.2
                    .id
                    .to_ascii_lowercase()
                    .cmp(&right.2.id.to_ascii_lowercase())
            })
            .then_with(|| left.1.cmp(&right.1))
    });

    let matched_package_count = matches.len();
    let packages = matches
        .into_iter()
        .take(limit)
        .map(|(_, _, package)| package)
        .collect::<Vec<_>>();
    let returned_package_count = packages.len();

    Ok(AddonIndexSearch {
        index_path,
        index_name,
        query,
        package_count,
        matched_package_count,
        returned_package_count,
        packages,
    })
}

fn search_score(
    package: &AddonIndexPackage,
    governance: Option<&super::governance::AddonIndexGovernance>,
    query: &str,
    query_tokens: &[String],
) -> Option<u8> {
    let searchable_fields = searchable_fields(package, governance);
    search_score_from_fields(&searchable_fields, query, query_tokens)
}

fn search_score_from_fields(
    searchable_fields: &[String],
    query: &str,
    query_tokens: &[String],
) -> Option<u8> {
    if query.is_empty() {
        return Some(2);
    }
    if searchable_fields
        .iter()
        .any(|field| field.eq_ignore_ascii_case(query))
    {
        return Some(0);
    }
    if searchable_fields
        .iter()
        .any(|field| field.starts_with(query))
    {
        return Some(1);
    }
    if query_tokens.is_empty() {
        return Some(2);
    }
    if query_tokens
        .iter()
        .all(|token| searchable_fields.iter().any(|field| field.contains(token)))
    {
        return Some(2);
    }

    None
}

fn searchable_fields(
    package: &AddonIndexPackage,
    governance: Option<&super::governance::AddonIndexGovernance>,
) -> Vec<String> {
    let governance_terms = governance
        .map(|governance| governance.searchable_terms_for_package(&package.id))
        .unwrap_or_default();
    build_searchable_fields(package, governance_terms)
}

fn build_searchable_fields(
    package: &AddonIndexPackage,
    governance_terms: Vec<String>,
) -> Vec<String> {
    let mut fields = vec![
        package.id.clone(),
        package.name.clone(),
        package.version.clone(),
        package.source.display_name(),
    ];
    if let Some(source_url) = package.source_url.as_deref() {
        fields.push(source_url.to_string());
    }
    if let Some(website_url) = package.website_url.as_deref() {
        fields.push(website_url.to_string());
    }
    fields.extend(package.match_package_ids.iter().cloned());
    fields.extend(package.addon_directories.iter().cloned());
    fields.extend(package.supported_flavors.iter().cloned());
    fields.extend(governance_terms);

    fields.into_iter().map(|value| normalize(&value)).collect()
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(normalize)
        .filter(|token| !token.is_empty())
        .collect()
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_flavor_alias(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

fn search_community_search_docs(
    index_path: PathBuf,
    index_name: String,
    query: String,
    limit: usize,
    packages: Vec<&CommunitySearchDoc>,
) -> AppResult<AddonIndexSearch> {
    let package_count = packages.len();
    let query = query.trim().to_string();
    let query_norm = normalize(&query);
    let query_tokens = tokenize_query(&query);
    let mut matches = packages
        .into_iter()
        .enumerate()
        .filter_map(|(ordinal, package)| {
            package
                .search_score(&query_norm, &query_tokens)
                .map(|score| (score, ordinal, package))
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.2.normalized_id.cmp(&right.2.normalized_id))
            .then_with(|| left.1.cmp(&right.1))
    });

    let matched_package_count = matches.len();
    let packages = matches
        .into_iter()
        .take(limit)
        .map(|(_, _, package)| package.package.clone())
        .collect::<Vec<_>>();
    let returned_package_count = packages.len();

    Ok(AddonIndexSearch {
        index_path,
        index_name,
        query,
        package_count,
        matched_package_count,
        returned_package_count,
        packages,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::core::addon::AddonSourceRef;

    #[test]
    fn search_addon_index_matches_all_query_terms() {
        let temp = tempdir().expect("temp dir");
        let index_path = temp.path().join("community-addon-index.toml");
        fs::write(
            &index_path,
            r#"
schema_version = 1
name = "Community Catalog"

[[packages]]
id = "elvui"
name = "ElvUI"
version = "15.13"
source = { kind = "tukui_addon", slug = "elvui" }
source_url = "https://api.tukui.org/v1/addon/elvui"
website_url = "https://tukui.org/elvui"
addon_directories = ["ElvUI", "ElvUI_Libraries", "ElvUI_Options"]
supported_flavors = ["retail", "classic", "classic-era"]

[[packages]]
id = "details"
name = "Details! Damage Meter"
version = "vdx1042w"
source = { kind = "wago_addon", project_id = "qv63A7Gb", release_id = "vdx1042w" }
source_url = "https://addons.wago.io/addons/qv63A7Gb"
website_url = "https://addons.wago.io/addons/qv63A7Gb"
addon_directories = ["Details", "Details_Streamer"]
supported_flavors = ["retail"]
"#,
        )
        .expect("index file");

        let result = search_addon_index(AddonIndexSearchRequest {
            index_path: index_path.clone(),
            query: "Elv UI".to_string(),
            limit: 10,
        })
        .expect("search");

        assert_eq!(result.index_path, index_path);
        assert_eq!(result.index_name, "Community Catalog");
        assert_eq!(result.package_count, 2);
        assert_eq!(result.matched_package_count, 1);
        assert_eq!(result.returned_package_count, 1);
        assert_eq!(result.packages[0].id, "elvui");
        assert_eq!(
            result.packages[0].source,
            AddonSourceRef::TukuiAddon {
                slug: "elvui".to_string(),
                version: None,
            }
        );
    }

    #[test]
    fn search_addon_index_matches_governance_aliases() {
        let temp = tempdir().expect("temp dir");
        let index_path = temp.path().join("community-addon-index.toml");
        fs::write(
            &index_path,
            r#"
schema_version = 1
name = "Community Catalog"

[[packages]]
id = "bigwigs"
name = "BigWigs"
version = "v414.9"
source = { kind = "github_release", owner = "BigWigsMods", repo = "BigWigs", tag = "v414.9", asset_name = "BigWigs-v414.9.zip" }
source_url = "https://github.com/BigWigsMods/BigWigs/releases/download/v414.9/BigWigs-v414.9.zip"
website_url = "https://github.com/BigWigsMods/BigWigs"
addon_directories = ["BigWigs"]
supported_flavors = ["retail"]
"#,
        )
        .expect("index file");
        fs::write(
            index_path.with_extension("governance.json"),
            r#"
{
  "schema_version": 1,
  "name": "Community Catalog Governance",
  "updated_at": "2026-05-06T00:00:00Z",
  "entries": [
    {
      "id": "bigwigs",
      "aliases": ["Big Wigs"],
      "upstream_hosts": ["github"],
      "source_attribution": "BigWigsMods/BigWigs official GitHub releases",
      "maintainer": "hearthsync",
      "status": "active",
      "confidence": "high",
      "last_verified_at": "2026-05-06T00:00:00Z",
      "notes": "Alias-driven discovery"
    }
  ]
}
"#,
        )
        .expect("governance file");

        let result = search_addon_index(AddonIndexSearchRequest {
            index_path,
            query: "Big Wigs".to_string(),
            limit: 10,
        })
        .expect("search");

        assert_eq!(result.matched_package_count, 1);
        assert_eq!(result.returned_package_count, 1);
        assert_eq!(result.packages[0].id, "bigwigs");
    }

    #[test]
    fn community_addon_index_cache_is_reused() {
        let first = community_addon_index_cache().expect("community cache");
        let second = community_addon_index_cache().expect("community cache");

        assert!(std::ptr::eq(first, second));
        assert!(!first.index_name.trim().is_empty());
        assert!(!first.packages.is_empty());
    }

    #[test]
    fn search_community_addon_index_matches_builtin_governance_aliases() {
        let result = search_community_addon_index("Big Wigs".to_string(), 10, WowFlavor::Retail)
            .expect("search community catalog");

        assert_eq!(
            result.index_path,
            PathBuf::from(COMMUNITY_ADDON_INDEX_LABEL)
        );
        assert_eq!(result.matched_package_count, 1);
        assert_eq!(result.returned_package_count, 1);
        assert_eq!(result.packages[0].id, "bigwigs");
    }

    #[test]
    fn search_community_addon_index_filters_by_flavor_aliases() {
        let classic_result =
            search_community_addon_index("ElvUI_Options".to_string(), 10, WowFlavor::ClassicEra)
                .expect("classic-era search");
        assert_eq!(classic_result.returned_package_count, 1);
        assert_eq!(classic_result.packages[0].id, "elvui");

        let retail_only_result =
            search_community_addon_index("BigWigs".to_string(), 10, WowFlavor::ClassicEra)
                .expect("classic-era search filters retail-only package");
        assert_eq!(retail_only_result.returned_package_count, 0);
    }

    #[test]
    fn search_addon_index_returns_all_packages_when_query_is_blank() {
        let temp = tempdir().expect("temp dir");
        let index_path = temp.path().join("community-addon-index.toml");
        fs::write(
            &index_path,
            r#"
schema_version = 1
name = "Community Catalog"

[[packages]]
id = "bigwigs"
name = "BigWigs"
version = "v414.9"
source = { kind = "github_release", owner = "BigWigsMods", repo = "BigWigs", tag = "v414.9", asset_name = "BigWigs-v414.9.zip" }
source_url = "https://github.com/BigWigsMods/BigWigs/releases/download/v414.9/BigWigs-v414.9.zip"
website_url = "https://github.com/BigWigsMods/BigWigs"
addon_directories = ["BigWigs"]
supported_flavors = ["retail"]
"#,
        )
        .expect("index file");

        let result = search_addon_index(AddonIndexSearchRequest {
            index_path,
            query: "   ".to_string(),
            limit: 10,
        })
        .expect("search");

        assert_eq!(result.matched_package_count, 1);
        assert_eq!(result.returned_package_count, 1);
        assert_eq!(result.packages[0].id, "bigwigs");
    }

    #[test]
    fn search_addon_index_respects_result_limit() {
        let temp = tempdir().expect("temp dir");
        let index_path = temp.path().join("community-addon-index.toml");
        fs::write(
            &index_path,
            r#"
schema_version = 1
name = "Community Catalog"

[[packages]]
id = "a"
name = "Alpha"
version = "1"
source = { kind = "tukui_addon", slug = "alpha" }
addon_directories = ["Alpha"]
supported_flavors = ["retail"]

[[packages]]
id = "b"
name = "Beta"
version = "1"
source = { kind = "tukui_addon", slug = "beta" }
addon_directories = ["Beta"]
supported_flavors = ["retail"]
"#,
        )
        .expect("index file");

        let result = search_addon_index(AddonIndexSearchRequest {
            index_path,
            query: "a".to_string(),
            limit: 1,
        })
        .expect("search");

        assert_eq!(result.matched_package_count, 2);
        assert_eq!(result.returned_package_count, 1);
        assert_eq!(result.packages.len(), 1);
        assert!(matches!(result.packages[0].id.as_str(), "a"));
    }
}
