use super::model::{CurseForgeFile, CurseForgeGame, CurseForgeGameVersionType};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurseForgeFileReleaseType {
    Stable,
    Beta,
    Alpha,
}

impl CurseForgeFileReleaseType {
    fn rank(self) -> u8 {
        match self {
            Self::Stable => 1,
            Self::Beta => 2,
            Self::Alpha => 3,
        }
    }
}

pub(crate) fn select_curseforge_version_type(
    version_types: &[CurseForgeGameVersionType],
    flavor: WowFlavor,
) -> AppResult<CurseForgeGameVersionType> {
    let candidates = curseforge_version_type_candidates(flavor);
    version_types
        .iter()
        .find(|version_type| {
            let slug = version_type.slug.to_ascii_lowercase();
            let name = version_type.name.to_ascii_lowercase();
            candidates.iter().any(|candidate| {
                slug == *candidate
                    || slug.contains(candidate)
                    || name == *candidate
                    || name.contains(candidate)
            })
        })
        .cloned()
        .ok_or_else(|| {
            AppError::Validation(format!(
                "CurseForge version type for flavor `{}` was not found. Available version types: {}",
                flavor.as_str(),
                version_types
                    .iter()
                    .map(|item| format!("{}({})", item.slug, item.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

pub(super) fn is_world_of_warcraft_game(game: &CurseForgeGame) -> bool {
    let name = game.name.to_ascii_lowercase();
    let slug = game.slug.to_ascii_lowercase();
    name == "world of warcraft"
        || slug == "world-of-warcraft"
        || slug == "world_of_warcraft"
        || (name.contains("world") && name.contains("warcraft"))
}

pub(crate) fn select_latest_curseforge_file(
    files: Vec<CurseForgeFile>,
    version_type_id: Option<u32>,
    max_release_type: Option<CurseForgeFileReleaseType>,
) -> AppResult<CurseForgeFile> {
    let mut candidates = files
        .into_iter()
        .filter(|file| file.is_available)
        .filter(|file| file.file_name.ends_with(".zip"))
        .filter(|file| {
            version_type_id.is_none_or(|version_type_id| {
                file_matches_curseforge_version_type(file, version_type_id)
            })
        })
        .filter(|file| {
            max_release_type.is_none_or(|max_release_type| {
                file_matches_curseforge_release_type(file, max_release_type)
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.file_date.cmp(&left.file_date));

    let Some(file) = candidates.into_iter().next() else {
        return Err(AppError::Validation(missing_curseforge_file_message(
            version_type_id,
            max_release_type,
        )));
    };

    validate_curseforge_file(file)
}

pub(super) fn ensure_curseforge_file_matches_version_type(
    file: &CurseForgeFile,
    version_type_id: u32,
) -> AppResult<()> {
    if file_matches_curseforge_version_type(file, version_type_id) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "CurseForge file `{}` does not match target version type `{version_type_id}`",
            file.id
        )))
    }
}

pub(crate) fn validate_curseforge_file(file: CurseForgeFile) -> AppResult<CurseForgeFile> {
    if !file.is_available {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` is not available for download",
            file.id
        )));
    }
    if !file.file_name.ends_with(".zip") {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` is not a `.zip` archive",
            file.file_name
        )));
    }
    if file.download_url.is_none() {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` does not provide a download URL",
            file.id
        )));
    }

    Ok(file)
}

fn curseforge_version_type_candidates(flavor: WowFlavor) -> &'static [&'static str] {
    match flavor {
        WowFlavor::Retail => &["wow_retail", "retail"],
        WowFlavor::Classic => &["wow_classic", "classic"],
        WowFlavor::ClassicEra => &["wow_classic_era", "classic_era", "classic era"],
        WowFlavor::Ptr => &["wow_ptr", "ptr"],
        WowFlavor::Beta => &["wow_beta", "beta"],
        WowFlavor::Xptr => &["wow_xptr", "xptr"],
    }
}

fn file_matches_curseforge_version_type(file: &CurseForgeFile, version_type_id: u32) -> bool {
    file.sortable_game_versions
        .iter()
        .any(|item| item.game_version_type_id == version_type_id)
}

fn file_matches_curseforge_release_type(
    file: &CurseForgeFile,
    max_release_type: CurseForgeFileReleaseType,
) -> bool {
    let Some(rank) = curseforge_file_release_rank(file.release_type) else {
        return false;
    };
    rank <= max_release_type.rank()
}

fn curseforge_file_release_rank(release_type: u8) -> Option<u8> {
    match release_type {
        1 => Some(CurseForgeFileReleaseType::Stable.rank()),
        2 => Some(CurseForgeFileReleaseType::Beta.rank()),
        3 => Some(CurseForgeFileReleaseType::Alpha.rank()),
        _ => None,
    }
}

fn missing_curseforge_file_message(
    version_type_id: Option<u32>,
    max_release_type: Option<CurseForgeFileReleaseType>,
) -> String {
    let release_suffix = match max_release_type {
        Some(CurseForgeFileReleaseType::Stable) => " for stable releases only",
        Some(CurseForgeFileReleaseType::Beta) => " for stable/beta releases",
        Some(CurseForgeFileReleaseType::Alpha) => " for stable/beta/alpha releases",
        None => "",
    };
    match version_type_id {
        Some(version_type_id) => format!(
            "CurseForge mod does not expose an available `.zip` file for version type `{version_type_id}`{release_suffix}"
        ),
        None => format!("CurseForge mod does not expose an available `.zip` file{release_suffix}"),
    }
}
