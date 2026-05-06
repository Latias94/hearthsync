use super::storage::inspect_addon_index;
use super::{AddonIndexPackage, AddonIndexSearch, AddonIndexSearchRequest};
use crate::core::error::AppResult;

pub fn search_addon_index(request: AddonIndexSearchRequest) -> AppResult<AddonIndexSearch> {
    let inspection = inspect_addon_index(&request.index_path)?;
    let query = request.query.trim().to_string();
    let query_norm = normalize(&query);
    let query_tokens = tokenize_query(&query);
    let mut matches = inspection
        .index
        .packages
        .into_iter()
        .enumerate()
        .filter_map(|(ordinal, package)| {
            search_score(&package, &query_norm, &query_tokens)
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
        .take(request.limit)
        .map(|(_, _, package)| package)
        .collect::<Vec<_>>();
    let returned_package_count = packages.len();

    Ok(AddonIndexSearch {
        index_path: inspection.index_path,
        index_name: inspection.index.name,
        query,
        package_count: inspection.package_count,
        matched_package_count,
        returned_package_count,
        packages,
    })
}

fn search_score(package: &AddonIndexPackage, query: &str, query_tokens: &[String]) -> Option<u8> {
    let searchable_fields = searchable_fields(package);
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

fn searchable_fields(package: &AddonIndexPackage) -> Vec<String> {
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
