use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use tempfile::tempdir;
use zip::ZipArchive;

use super::super::addon_lock::ExtractedAddonLock;
use super::super::constants::{ADDON_LOCK_ENTRY, ADDON_SOURCE_INDEX_ENTRY};
use super::super::shared::addon_source_index::{BundleAddonSourceEntry, BundleAddonSourceIndex};
use super::super::shared::path::join_segments;
use super::safety::reject_unsupported_bundle_symlink_entry;
use crate::core::addon::lock::AddonLockSourceOverride;
use crate::core::archive_io::copy_reader_to_path;
use crate::core::archive_path::{safe_zip_segments_under, validate_portable_path_segment};
use crate::core::boundary_validation::is_sha256_hex;
use crate::core::error::{AppError, AppResult};

pub(in crate::core::bundle) fn extract_embedded_addon_lock(
    bundle_path: &Path,
) -> AppResult<ExtractedAddonLock> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let stage_dir = tempdir()?;
    let lock_path = stage_dir.path().join("lock.toml");
    {
        let mut lock_entry = archive.by_name(ADDON_LOCK_ENTRY).map_err(|_| {
            AppError::NotFound(format!(
                "bundle does not contain embedded addon lock `{ADDON_LOCK_ENTRY}`"
            ))
        })?;
        reject_unsupported_bundle_symlink_entry(
            lock_entry.name(),
            lock_entry.is_symlink(),
            lock_entry.is_dir(),
        )?;
        copy_reader_to_path(&mut lock_entry, &lock_path)?;
    }

    let source_overrides = extract_bundle_addon_source_overrides(&mut archive, stage_dir.path())?;

    Ok(ExtractedAddonLock {
        lock_path,
        source_overrides,
        _stage_dir: stage_dir,
    })
}

fn extract_bundle_addon_source_overrides(
    archive: &mut ZipArchive<File>,
    stage_root: &Path,
) -> AppResult<Vec<AddonLockSourceOverride>> {
    let source_index = match archive.by_name(ADDON_SOURCE_INDEX_ENTRY) {
        Ok(mut entry) => {
            reject_unsupported_bundle_symlink_entry(
                entry.name(),
                entry.is_symlink(),
                entry.is_dir(),
            )?;
            let mut content = String::new();
            entry.read_to_string(&mut content)?;
            toml::from_str::<BundleAddonSourceIndex>(&content)?
        }
        Err(zip::result::ZipError::FileNotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    validate_bundle_addon_source_index(&source_index)?;

    let mut source_overrides = Vec::new();
    for source in source_index.sources {
        let segments = safe_bundle_addon_source_segments(&source.path)?;

        let archive_entry_name = format!("metadata/addons/{}", segments.join("/"));
        let mut source_entry = archive.by_name(&archive_entry_name).map_err(|_| {
            AppError::NotFound(format!(
                "bundle addon source archive is missing: {archive_entry_name}"
            ))
        })?;
        reject_unsupported_bundle_symlink_entry(
            source_entry.name(),
            source_entry.is_symlink(),
            source_entry.is_dir(),
        )?;
        let extracted_path = join_segments(stage_root, &segments);
        copy_reader_to_path(&mut source_entry, &extracted_path)?;

        source_overrides.push(AddonLockSourceOverride {
            comparison_key: source.comparison_key,
            archive_path: extracted_path,
        });
    }

    Ok(source_overrides)
}

fn validate_bundle_addon_source_index(source_index: &BundleAddonSourceIndex) -> AppResult<()> {
    if source_index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported bundle addon source index schema version: {}",
            source_index.schema_version
        )));
    }
    if source_index.sources.is_empty() {
        return Err(AppError::Validation(
            "bundle addon source index must contain at least one source".to_string(),
        ));
    }

    let mut comparison_keys = BTreeSet::new();
    let mut source_paths = BTreeSet::new();
    for source in &source_index.sources {
        validate_bundle_addon_source_entry(source, &mut source_paths)?;
        if !comparison_keys.insert(source.comparison_key.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate bundle addon source index comparison key: {}",
                source.comparison_key
            )));
        }
    }

    Ok(())
}

fn validate_bundle_addon_source_entry(
    source: &BundleAddonSourceEntry,
    source_paths: &mut BTreeSet<String>,
) -> AppResult<()> {
    if source.comparison_key.trim().is_empty() {
        return Err(AppError::Validation(
            "bundle addon source index comparison key must not be empty".to_string(),
        ));
    }
    if source.comparison_key.trim() != source.comparison_key {
        return Err(AppError::Validation(format!(
            "bundle addon source index comparison key must not have surrounding whitespace: {}",
            source.comparison_key
        )));
    }
    if source.package_id.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "bundle addon source index package id must not be empty for `{}`",
            source.comparison_key
        )));
    }

    let segments = safe_bundle_addon_source_segments(&source.path)?;
    let normalized_path = segments.join("/").to_ascii_lowercase();
    if !source_paths.insert(normalized_path) {
        return Err(AppError::Validation(format!(
            "duplicate bundle addon source path: {}",
            source.path
        )));
    }

    if !is_sha256_hex(&source.content_sha256) {
        return Err(AppError::Validation(format!(
            "bundle addon source index `{}` content_sha256 must be a 64-character SHA-256 hex digest",
            source.comparison_key
        )));
    }
    if source.addon_directories.is_empty() {
        return Err(AppError::Validation(format!(
            "bundle addon source index `{}` must declare at least one addon directory",
            source.comparison_key
        )));
    }

    let mut addon_directories = BTreeSet::new();
    for addon_directory in &source.addon_directories {
        validate_portable_path_segment(addon_directory, "addon directory").map_err(|error| {
            AppError::Validation(format!(
                "{error} for bundle addon source `{}`",
                source.comparison_key
            ))
        })?;
        let normalized = addon_directory.trim().to_ascii_lowercase();
        if !addon_directories.insert(normalized) {
            return Err(AppError::Validation(format!(
                "duplicate addon directory `{}` in bundle addon source `{}`",
                addon_directory, source.comparison_key
            )));
        }
    }

    Ok(())
}

fn safe_bundle_addon_source_segments(path: &str) -> AppResult<Vec<&str>> {
    safe_zip_segments_under(path, "sources", "bundle addon source path")
}

#[cfg(test)]
mod tests {
    use super::{safe_bundle_addon_source_segments, validate_bundle_addon_source_index};
    use crate::core::bundle::shared::addon_source_index::{
        BundleAddonSourceEntry, BundleAddonSourceIndex,
    };

    #[test]
    fn safe_bundle_addon_source_segments_rejects_non_portable_paths() {
        let error = safe_bundle_addon_source_segments("sources/CON.zip")
            .expect_err("non-portable bundle addon source path should fail");

        assert!(
            error
                .to_string()
                .contains("unsafe bundle addon source path")
        );
    }

    #[test]
    fn safe_bundle_addon_source_segments_requires_sources_root() {
        let error = safe_bundle_addon_source_segments("archives/WeakAuras.zip")
            .expect_err("wrong root should fail");

        assert!(
            error
                .to_string()
                .contains("bundle addon source path must be under `sources/`")
        );
    }

    #[test]
    fn safe_bundle_addon_source_segments_accepts_portable_paths() {
        assert_eq!(
            safe_bundle_addon_source_segments("sources/providers/WeakAuras.zip")
                .expect("portable bundle addon source path"),
            vec!["sources", "providers", "WeakAuras.zip"]
        );
    }

    #[test]
    fn bundle_addon_source_index_rejects_invalid_state_contracts() {
        let duplicate_keys = BundleAddonSourceIndex {
            sources: vec![
                valid_source_entry(),
                BundleAddonSourceEntry {
                    path: "sources/WeakAuras-copy.zip".to_string(),
                    ..valid_source_entry()
                },
            ],
            ..valid_source_index()
        };
        let mut duplicate_paths = valid_source_index();
        duplicate_paths.sources.push(BundleAddonSourceEntry {
            comparison_key: "addons:plater".to_string(),
            package_id: "plater".to_string(),
            path: "sources/WEAKAURAS.zip".to_string(),
            content_sha256: valid_hash(),
            addon_directories: vec!["Plater".to_string()],
        });
        let mut invalid_hash = valid_source_index();
        invalid_hash.sources[0].content_sha256 = "not-a-hash".to_string();
        let mut duplicate_addons = valid_source_index();
        duplicate_addons.sources[0].addon_directories =
            vec!["WeakAuras".to_string(), "weakauras".to_string()];

        for (case_name, index, expected_message) in [
            (
                "unsupported schema",
                BundleAddonSourceIndex {
                    schema_version: 2,
                    ..valid_source_index()
                },
                "unsupported bundle addon source index schema version",
            ),
            (
                "empty sources",
                BundleAddonSourceIndex {
                    sources: Vec::new(),
                    ..valid_source_index()
                },
                "must contain at least one source",
            ),
            (
                "blank comparison key",
                index_with_entry(BundleAddonSourceEntry {
                    comparison_key: " ".to_string(),
                    ..valid_source_entry()
                }),
                "comparison key must not be empty",
            ),
            (
                "duplicate comparison key",
                duplicate_keys,
                "duplicate bundle addon source index comparison key",
            ),
            (
                "blank package id",
                index_with_entry(BundleAddonSourceEntry {
                    package_id: " ".to_string(),
                    ..valid_source_entry()
                }),
                "package id must not be empty",
            ),
            (
                "unsafe source path",
                index_with_entry(BundleAddonSourceEntry {
                    path: "sources/CON.zip".to_string(),
                    ..valid_source_entry()
                }),
                "unsafe bundle addon source path",
            ),
            (
                "duplicate source path",
                duplicate_paths,
                "duplicate bundle addon source path",
            ),
            (
                "invalid hash",
                invalid_hash,
                "content_sha256 must be a 64-character SHA-256 hex digest",
            ),
            (
                "empty addon directories",
                index_with_entry(BundleAddonSourceEntry {
                    addon_directories: Vec::new(),
                    ..valid_source_entry()
                }),
                "must declare at least one addon directory",
            ),
            (
                "non-portable addon directory",
                index_with_entry(BundleAddonSourceEntry {
                    addon_directories: vec!["CON".to_string()],
                    ..valid_source_entry()
                }),
                "invalid addon directory name",
            ),
            (
                "duplicate addon directories",
                duplicate_addons,
                "duplicate addon directory",
            ),
        ] {
            let error = validate_bundle_addon_source_index(&index).expect_err(case_name);

            assert!(
                error.to_string().contains(expected_message),
                "{case_name}: expected `{expected_message}`, got `{error}`"
            );
        }
    }

    fn valid_source_index() -> BundleAddonSourceIndex {
        BundleAddonSourceIndex {
            schema_version: 1,
            sources: vec![valid_source_entry()],
        }
    }

    fn valid_source_entry() -> BundleAddonSourceEntry {
        BundleAddonSourceEntry {
            comparison_key: "addons:weakauras".to_string(),
            package_id: "weakauras".to_string(),
            path: "sources/WeakAuras.zip".to_string(),
            content_sha256: valid_hash(),
            addon_directories: vec!["WeakAuras".to_string()],
        }
    }

    fn index_with_entry(entry: BundleAddonSourceEntry) -> BundleAddonSourceIndex {
        BundleAddonSourceIndex {
            schema_version: 1,
            sources: vec![entry],
        }
    }

    fn valid_hash() -> String {
        "a".repeat(64)
    }
}
