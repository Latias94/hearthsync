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

#[cfg(test)]
mod tests {
    use super::super::model::{
        CurseForgeFileDependency, CurseForgeFileHash, CurseForgeSortableGameVersion,
    };
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

    #[test]
    fn validate_curseforge_file_rejects_missing_download_url() {
        let error = validate_curseforge_file(CurseForgeFile {
            download_url: None,
            ..curseforge_file(
                1,
                "addon.zip",
                "2026-04-02T12:00:00Z",
                "https://example.com/addon.zip",
                517,
                1,
            )
        })
        .expect_err("missing download url");

        assert!(error.to_string().contains("download URL"));
    }

    #[test]
    fn validate_curseforge_file_rejects_non_portable_file_name() {
        let error = validate_curseforge_file(curseforge_file(
            1,
            "bad/name.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        ))
        .expect_err("non-portable filename");

        assert!(error.to_string().contains("invalid CurseForge file name"));
    }

    #[test]
    fn validate_curseforge_file_rejects_invalid_download_url() {
        let error = validate_curseforge_file(curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "ftp://example.com/addon.zip",
            517,
            1,
        ))
        .expect_err("invalid download url");

        assert!(error.to_string().contains("download URL must start with"));
    }

    #[test]
    fn validate_curseforge_file_rejects_invalid_file_date() {
        let error = validate_curseforge_file(curseforge_file(
            1,
            "addon.zip",
            "not-a-timestamp",
            "https://example.com/addon.zip",
            517,
            1,
        ))
        .expect_err("invalid file date");

        assert!(error.to_string().contains("file date must be"));
    }

    #[test]
    fn validate_curseforge_file_rejects_invalid_remote_validator_metadata() {
        let mut zero_length = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        zero_length.file_length = Some(0);

        let mut short_sha1 = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        short_sha1.hashes = vec![CurseForgeFileHash {
            value: "abc".to_string(),
            algo: 1,
        }];

        let mut short_md5 = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        short_md5.hashes = vec![CurseForgeFileHash {
            value: "abc".to_string(),
            algo: 2,
        }];

        let mut duplicate_sha1 = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        duplicate_sha1.hashes = vec![
            CurseForgeFileHash {
                value: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                algo: 1,
            },
            CurseForgeFileHash {
                value: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
                algo: 1,
            },
        ];

        let mut zero_version_type = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        zero_version_type.sortable_game_versions = vec![CurseForgeSortableGameVersion {
            game_version_type_id: 0,
        }];

        for (file, expected_message) in [
            (zero_length, "file length must be greater than zero"),
            (
                short_sha1,
                "hash value for algo `1` must be a 40-character SHA-1 hex digest",
            ),
            (
                short_md5,
                "hash value for algo `2` must be a 32-character MD5 hex digest",
            ),
            (duplicate_sha1, "declares duplicate hash algo `1`"),
            (
                zero_version_type,
                "sortable game version type id must be greater than zero",
            ),
        ] {
            let error =
                validate_curseforge_file(file).expect_err("invalid remote validator metadata");
            assert!(
                error.to_string().contains(expected_message),
                "expected `{}` in `{}`",
                expected_message,
                error
            );
        }
    }

    #[test]
    fn validate_curseforge_file_rejects_invalid_policy_and_dependency_metadata() {
        let mut invalid_release_type = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            4,
        );
        invalid_release_type.dependencies = vec![CurseForgeFileDependency {
            mod_id: 99,
            relation_type: 3,
        }];

        let mut zero_dependency_mod_id = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        zero_dependency_mod_id.dependencies = vec![CurseForgeFileDependency {
            mod_id: 0,
            relation_type: 3,
        }];

        let mut zero_dependency_relation_type = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        zero_dependency_relation_type.dependencies = vec![CurseForgeFileDependency {
            mod_id: 99,
            relation_type: 0,
        }];

        let mut unknown_positive_dependency_relation_type = curseforge_file(
            1,
            "addon.zip",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        );
        unknown_positive_dependency_relation_type.dependencies = vec![CurseForgeFileDependency {
            mod_id: 99,
            relation_type: 9,
        }];

        for (file, expected_message) in [
            (invalid_release_type, "release type must be one of"),
            (
                zero_dependency_mod_id,
                "dependency mod id must be greater than zero",
            ),
            (
                zero_dependency_relation_type,
                "dependency relation type must be greater than zero",
            ),
        ] {
            let error =
                validate_curseforge_file(file).expect_err("invalid policy/dependency metadata");
            assert!(
                error.to_string().contains(expected_message),
                "expected `{}` in `{}`",
                expected_message,
                error
            );
        }

        validate_curseforge_file(unknown_positive_dependency_relation_type)
            .expect("unknown positive dependency relation type remains ignorable");
    }

    #[test]
    fn validate_curseforge_file_accepts_uppercase_zip_extension() {
        let file = validate_curseforge_file(curseforge_file(
            1,
            "addon.ZIP",
            "2026-04-02T12:00:00Z",
            "https://example.com/addon.zip",
            517,
            1,
        ))
        .expect("uppercase zip");

        assert_eq!(file.file_name, "addon.ZIP");
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
