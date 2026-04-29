mod api;
mod model;
mod policy;
mod select;

use api::{
    fetch_curseforge_file_with_client, fetch_curseforge_mod_files_with_client,
    resolve_curseforge_wow_context_with_client,
    search_curseforge_mods_with_client as search_curseforge_mod_payloads_with_client,
};
use model::CurseForgeSearchMod;
#[allow(unused_imports)]
pub(crate) use model::{
    CurseForgeFile, CurseForgeFileDependency, CurseForgeFileHash, CurseForgeGameVersionType,
    CurseForgeSortableGameVersion,
};
pub(crate) use policy::CurseForgeFileReleaseType;
pub(super) use policy::{
    remote_validators_for_curseforge_file, required_dependency_mod_ids_for_curseforge_file,
};
use select::ensure_curseforge_file_matches_version_type;
#[allow(unused_imports)]
pub(crate) use select::{
    select_curseforge_version_type, select_latest_curseforge_file, validate_curseforge_file,
};

use super::http::HttpClient;
use super::{AddonSearchResult, AddonSourceRef};
use crate::core::error::AppResult;
use crate::core::install::WowFlavor;

pub(super) fn resolve_curseforge_file_with_client(
    client: &impl HttpClient,
    mod_id: u32,
    file_id: Option<u32>,
    target_flavor: Option<WowFlavor>,
    max_release_type: Option<CurseForgeFileReleaseType>,
) -> AppResult<CurseForgeFile> {
    let wow_context = target_flavor
        .map(|flavor| resolve_curseforge_wow_context_with_client(client, flavor))
        .transpose()?;
    if let Some(file_id) = file_id {
        let file = fetch_curseforge_file_with_client(client, mod_id, file_id)?;
        if let Some(wow_context) = &wow_context {
            ensure_curseforge_file_matches_version_type(&file, wow_context.version_type_id)?;
        }
        return validate_curseforge_file(file);
    }

    let files = fetch_curseforge_mod_files_with_client(client, mod_id)?;
    select_latest_curseforge_file(
        files,
        wow_context.as_ref().map(|item| item.version_type_id),
        max_release_type,
    )
}

pub(super) fn search_curseforge_mods_with_client(
    client: &impl HttpClient,
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<Vec<AddonSearchResult>> {
    let (wow_context, mods) =
        search_curseforge_mod_payloads_with_client(client, query, flavor, limit)?;
    Ok(mods
        .into_iter()
        .map(|mod_item| to_addon_search_result(mod_item, wow_context.version_type_id))
        .collect())
}

fn to_addon_search_result(
    mod_item: CurseForgeSearchMod,
    version_type_id: u32,
) -> AddonSearchResult {
    let matched_index = mod_item
        .latest_files_indexes
        .iter()
        .find(|item| item.game_version_type_id == version_type_id);
    let file_id = matched_index.map(|item| item.file_id);
    let source = AddonSourceRef::CurseForgeMod {
        mod_id: mod_item.id,
        file_id,
    };
    let install_hint = source.display_name();

    AddonSearchResult {
        provider: "curseforge",
        name: mod_item.name,
        summary: normalize_optional_provider_text(mod_item.summary),
        source,
        install_hint,
        website_url: mod_item.links.website_url,
        provider_project_id: Some(mod_item.id),
        provider_file_id: file_id,
        download_count: mod_item.download_count,
    }
}

fn normalize_optional_provider_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::model::{CurseForgeFileIndex, CurseForgeSearchMod, CurseForgeSearchModLinks};
    use super::*;

    #[test]
    fn to_addon_search_result_projects_matching_file_index_and_normalizes_summary() {
        let result = to_addon_search_result(
            CurseForgeSearchMod {
                id: 42,
                name: "WeakAuras".to_string(),
                summary: Some("  Aura tracking  ".to_string()),
                download_count: 100,
                latest_files_indexes: vec![
                    CurseForgeFileIndex {
                        file_id: 777,
                        game_version_type_id: 517,
                    },
                    CurseForgeFileIndex {
                        file_id: 778,
                        game_version_type_id: 517,
                    },
                ],
                links: CurseForgeSearchModLinks {
                    website_url: Some(
                        "https://www.curseforge.com/wow/addons/weakauras-2".to_string(),
                    ),
                },
            },
            517,
        );

        assert_eq!(result.provider, "curseforge");
        assert_eq!(result.name, "WeakAuras");
        assert_eq!(result.summary.as_deref(), Some("Aura tracking"));
        assert_eq!(
            result.source,
            AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: Some(777),
            }
        );
        assert_eq!(result.install_hint, "curseforge:42@777");
        assert_eq!(result.provider_project_id, Some(42));
        assert_eq!(result.provider_file_id, Some(777));
        assert_eq!(result.download_count, 100);
    }

    #[test]
    fn to_addon_search_result_drops_blank_summary_and_absent_version_match() {
        let result = to_addon_search_result(
            CurseForgeSearchMod {
                id: 43,
                name: "SharedMedia".to_string(),
                summary: Some("   ".to_string()),
                download_count: 50,
                latest_files_indexes: Vec::new(),
                links: CurseForgeSearchModLinks { website_url: None },
            },
            517,
        );

        assert_eq!(result.summary, None);
        assert_eq!(
            result.source,
            AddonSourceRef::CurseForgeMod {
                mod_id: 43,
                file_id: None,
            }
        );
        assert_eq!(result.install_hint, "curseforge:43");
        assert_eq!(result.provider_file_id, None);
    }
}
