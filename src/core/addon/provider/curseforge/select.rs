use std::collections::BTreeSet;

use super::model::{CurseForgeFile, CurseForgeGame, CurseForgeGameVersionType};
use super::policy::{
    CurseForgeFileReleaseType, curseforge_hash_contract, file_matches_curseforge_release_type,
    validate_curseforge_file_dependencies, validate_curseforge_release_type,
};
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::boundary_validation::{
    is_rfc3339_timestamp_shape, validate_hex_digest, validate_http_url,
};
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

pub(crate) fn validate_curseforge_file(file: CurseForgeFile) -> AppResult<CurseForgeFile> {
    validate_curseforge_file_metadata(&file)?;
    if !file.is_available {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` is not available for download",
            file.id
        )));
    }
    if !is_zip_file_name(&file.file_name) {
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

pub(crate) fn validate_curseforge_file_metadata(file: &CurseForgeFile) -> AppResult<()> {
    if file.id == 0 {
        return Err(AppError::Validation(
            "CurseForge file id must be greater than zero".to_string(),
        ));
    }
    validate_portable_path_segment(&file.file_name, "CurseForge file")?;
    if !is_rfc3339_timestamp_shape(&file.file_date) {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` file date must be an RFC 3339 timestamp",
            file.id
        )));
    }
    if let Some(download_url) = &file.download_url {
        validate_curseforge_download_url(file.id, download_url)?;
    }
    if file.file_length.is_some_and(|file_length| file_length == 0) {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` file length must be greater than zero",
            file.id
        )));
    }
    validate_curseforge_release_type(file)?;
    validate_curseforge_file_dependencies(file)?;
    validate_curseforge_file_hashes(file)?;
    validate_curseforge_sortable_game_versions(file)?;

    Ok(())
}

fn validate_curseforge_download_url(file_id: u32, download_url: &str) -> AppResult<()> {
    validate_http_url(
        download_url,
        &format!("CurseForge file `{file_id}` download URL"),
    )
}

fn is_zip_file_name(file_name: &str) -> bool {
    file_name.to_ascii_lowercase().ends_with(".zip")
}

fn validate_curseforge_file_hashes(file: &CurseForgeFile) -> AppResult<()> {
    let mut known_hash_algos = BTreeSet::new();
    for hash in &file.hashes {
        if hash.value.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "CurseForge file `{}` hash value for algo `{}` must not be empty",
                file.id, hash.algo
            )));
        }
        if hash.value.trim() != hash.value {
            return Err(AppError::Validation(format!(
                "CurseForge file `{}` hash value for algo `{}` must not have surrounding whitespace",
                file.id, hash.algo
            )));
        }
        let Some((expected_len, algorithm)) = curseforge_hash_contract(hash.algo) else {
            continue;
        };
        if !known_hash_algos.insert(hash.algo) {
            return Err(AppError::Validation(format!(
                "CurseForge file `{}` declares duplicate hash algo `{}`",
                file.id, hash.algo
            )));
        }
        validate_hex_digest(
            &hash.value,
            &format!(
                "CurseForge file `{}` hash value for algo `{}`",
                file.id, hash.algo
            ),
            expected_len,
            algorithm,
        )?;
    }

    Ok(())
}

fn validate_curseforge_sortable_game_versions(file: &CurseForgeFile) -> AppResult<()> {
    for game_version in &file.sortable_game_versions {
        if game_version.game_version_type_id == 0 {
            return Err(AppError::Validation(format!(
                "CurseForge file `{}` sortable game version type id must be greater than zero",
                file.id
            )));
        }
    }

    Ok(())
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
