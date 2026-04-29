use super::curseforge::{
    CurseForgeFileReleaseType, required_dependency_mod_ids_for_curseforge_file,
    resolve_curseforge_file_with_client, search_curseforge_mods_with_client,
};
use super::http::HttpClient;
use super::source::source_kind_label;
use super::{
    AddonProviderContext, AddonSearchResult, AddonSourceRef, AddonSourceResolutionPolicy,
    ResolvedAddonDependencies,
};
use crate::core::addon::policy::AddonReleaseChannel;
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

pub(super) fn search_addons_impl(
    http_client: &impl HttpClient,
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<Vec<AddonSearchResult>> {
    search_curseforge_mods_with_client(http_client, query, flavor, limit)
}

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
