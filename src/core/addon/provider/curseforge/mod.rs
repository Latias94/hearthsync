mod api;
mod model;
mod select;

use api::{
    fetch_curseforge_file_with_client, fetch_curseforge_mod_files_with_client,
    resolve_curseforge_wow_context_with_client,
    search_curseforge_mods_with_client as search_curseforge_mod_payloads_with_client,
};
use model::CurseForgeSearchMod;
#[allow(unused_imports)]
pub(crate) use model::{
    CurseForgeFile, CurseForgeFileDependency, CurseForgeGameVersionType,
    CurseForgeSortableGameVersion,
};
use select::ensure_curseforge_file_matches_version_type;
#[allow(unused_imports)]
pub(crate) use select::{
    CurseForgeFileReleaseType, select_curseforge_version_type, select_latest_curseforge_file,
    validate_curseforge_file,
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
        summary: mod_item.summary,
        source,
        install_hint,
        website_url: mod_item.links.website_url,
        provider_project_id: Some(mod_item.id),
        provider_file_id: file_id,
        download_count: mod_item.download_count,
    }
}
