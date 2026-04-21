use std::fs::File;
use std::io::Read;
use std::path::Path;

use tempfile::tempdir;
use zip::ZipArchive;

use super::super::addon_lock::ExtractedAddonLock;
use super::super::constants::{ADDON_LOCK_ENTRY, ADDON_SOURCE_INDEX_ENTRY};
use super::super::shared::addon_source_index::BundleAddonSourceIndex;
use super::super::shared::path::join_segments;
use super::safety::reject_unsupported_bundle_symlink_entry;
use crate::core::addon::lock::AddonLockSourceOverride;
use crate::core::archive_io::copy_reader_to_path;
use crate::core::archive_path::safe_zip_segments_under;
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

    if source_index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported bundle addon source index schema version: {}",
            source_index.schema_version
        )));
    }

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

fn safe_bundle_addon_source_segments(path: &str) -> AppResult<Vec<&str>> {
    safe_zip_segments_under(path, "sources", "bundle addon source path")
}

#[cfg(test)]
mod tests {
    use super::safe_bundle_addon_source_segments;

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
}
