use std::collections::BTreeSet;

use super::model::CurseForgeFile;
use super::policy::{
    curseforge_hash_contract, validate_curseforge_file_dependencies,
    validate_curseforge_release_type,
};
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::boundary_validation::{
    is_rfc3339_timestamp_shape, validate_hex_digest, validate_http_url,
};
use crate::core::error::{AppError, AppResult};
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

pub(super) fn is_zip_file_name(file_name: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::super::model::{
        CurseForgeFileDependency, CurseForgeFileHash, CurseForgeSortableGameVersion,
    };
    use super::*;
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
