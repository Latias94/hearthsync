use super::file_validation::{is_zip_file_name, validate_curseforge_file};
use super::model::{CurseForgeFile, CurseForgeGame, CurseForgeGameVersionType};
use super::policy::{CurseForgeFileReleaseType, file_matches_curseforge_release_type};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;
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
        .filter(|file| is_zip_file_name(&file.file_name))
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

#[cfg(test)]
mod tests {
    use super::super::model::CurseForgeSortableGameVersion;
    use super::*;
    #[test]
    fn select_latest_curseforge_file_prefers_newest_available_zip() {
        let file = select_latest_curseforge_file(
            vec![
                curseforge_file(
                    1,
                    "addon-old.zip",
                    "2026-04-01T12:00:00Z",
                    "https://example.com/old.zip",
                    517,
                    1,
                ),
                curseforge_file(
                    2,
                    "addon-new.zip",
                    "2026-04-02T12:00:00Z",
                    "https://example.com/new.zip",
                    517,
                    1,
                ),
                curseforge_file(
                    3,
                    "addon.txt",
                    "2026-04-03T12:00:00Z",
                    "https://example.com/skip.txt",
                    517,
                    1,
                ),
            ],
            None,
            None,
        )
        .expect("latest file");

        assert_eq!(file.id, 2);
        assert_eq!(file.file_name, "addon-new.zip");
    }

    #[test]
    fn select_latest_curseforge_file_filters_by_version_type() {
        let file = select_latest_curseforge_file(
            vec![
                curseforge_file(
                    1,
                    "addon-retail.zip",
                    "2026-04-01T12:00:00Z",
                    "https://example.com/retail.zip",
                    517,
                    1,
                ),
                curseforge_file(
                    2,
                    "addon-classic.zip",
                    "2026-04-02T12:00:00Z",
                    "https://example.com/classic.zip",
                    775,
                    1,
                ),
            ],
            Some(517),
            None,
        )
        .expect("latest file");

        assert_eq!(file.id, 1);
    }

    #[test]
    fn select_latest_curseforge_file_respects_release_channel_limit() {
        let file = select_latest_curseforge_file(
            vec![
                curseforge_file(
                    1,
                    "addon-stable.zip",
                    "2026-04-01T12:00:00Z",
                    "https://example.com/stable.zip",
                    517,
                    1,
                ),
                curseforge_file(
                    2,
                    "addon-beta.zip",
                    "2026-04-02T12:00:00Z",
                    "https://example.com/beta.zip",
                    517,
                    2,
                ),
                curseforge_file(
                    3,
                    "addon-alpha.zip",
                    "2026-04-03T12:00:00Z",
                    "https://example.com/alpha.zip",
                    517,
                    3,
                ),
            ],
            Some(517),
            Some(CurseForgeFileReleaseType::Beta),
        )
        .expect("latest allowed file");

        assert_eq!(file.id, 2);
        assert_eq!(file.file_name, "addon-beta.zip");
    }

    #[test]
    fn select_curseforge_version_type_matches_retail_slug() {
        let version_type = select_curseforge_version_type(
            &[
                CurseForgeGameVersionType {
                    id: 775,
                    name: "WoW Classic".to_string(),
                    slug: "wow_classic".to_string(),
                },
                CurseForgeGameVersionType {
                    id: 517,
                    name: "WoW Retail".to_string(),
                    slug: "wow_retail".to_string(),
                },
            ],
            WowFlavor::Retail,
        )
        .expect("version type");

        assert_eq!(version_type.id, 517);
    }
    fn curseforge_file(
        id: u32,
        file_name: &str,
        file_date: &str,
        download_url: &str,
        game_version_type_id: u32,
        release_type: u8,
    ) -> CurseForgeFile {
        CurseForgeFile {
            id,
            file_name: file_name.to_string(),
            file_date: file_date.to_string(),
            download_url: Some(download_url.to_string()),
            is_available: true,
            release_type,
            dependencies: Vec::new(),
            hashes: Vec::new(),
            file_length: None,
            sortable_game_versions: vec![CurseForgeSortableGameVersion {
                game_version_type_id,
            }],
        }
    }
}
