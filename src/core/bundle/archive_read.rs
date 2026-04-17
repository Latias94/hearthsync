use std::fs::File;
use std::io::Read;
use std::path::Path;

use tempfile::tempdir;
use zip::ZipArchive;

use super::*;
use crate::core::archive_io::copy_reader_to_path;

pub(super) fn collect_bundle_entry_names(bundle_path: &Path) -> AppResult<Vec<String>> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entry_names = Vec::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        entry_names.push(entry.name().to_string());
    }

    Ok(entry_names)
}

pub(super) fn read_bundle_entry_bytes_from_archive(
    archive: &mut ZipArchive<File>,
    archive_name: &str,
) -> AppResult<Vec<u8>> {
    let mut entry = archive
        .by_name(archive_name)
        .map_err(|_| AppError::NotFound(format!("bundle entry is missing: {archive_name}")))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub(super) fn extract_archive_entry_to_path(
    archive: &mut ZipArchive<File>,
    archive_name: &str,
    destination: &Path,
) -> AppResult<()> {
    let segments = safe_zip_segments(archive_name)?;
    if segments.is_empty() {
        return Err(AppError::Validation(format!(
            "bundle entry cannot be materialized because its path is empty: {archive_name}"
        )));
    }
    let mut entry = archive
        .by_name(archive_name)
        .map_err(|_| AppError::NotFound(format!("bundle entry is missing: {archive_name}")))?;
    copy_reader_to_path(&mut entry, destination)
}

pub(super) fn extract_embedded_addon_lock(bundle_path: &Path) -> AppResult<ExtractedAddonLock> {
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
        let segments = safe_zip_segments(&source.path)?;
        if segments.first().copied() != Some("sources") || segments.len() < 2 {
            return Err(AppError::Validation(format!(
                "bundle addon source path must be under `sources/`: {}",
                source.path
            )));
        }

        let archive_entry_name = format!("metadata/addons/{}", segments.join("/"));
        let mut source_entry = archive.by_name(&archive_entry_name).map_err(|_| {
            AppError::NotFound(format!(
                "bundle addon source archive is missing: {archive_entry_name}"
            ))
        })?;
        let extracted_path = join_segments(stage_root, &segments);
        copy_reader_to_path(&mut source_entry, &extracted_path)?;

        source_overrides.push(AddonLockSourceOverride {
            comparison_key: source.comparison_key,
            archive_path: extracted_path,
        });
    }

    Ok(source_overrides)
}

pub(super) fn read_manifest_from_archive(
    archive: &mut ZipArchive<File>,
) -> AppResult<BundleManifest> {
    let mut manifest_file = archive.by_name(MANIFEST_ENTRY)?;
    let mut content = String::new();
    manifest_file.read_to_string(&mut content)?;
    Ok(toml::from_str(&content)?)
}

pub(super) fn count_bundle_entries(archive: &mut ZipArchive<File>) -> AppResult<BundleEntryCounts> {
    let mut counts = BundleEntryCounts::default();

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }

        counts.total_files += 1;
        let name = file.name();
        if name == MANIFEST_ENTRY || name.starts_with("metadata/") {
            counts.metadata += 1;
        } else if name.starts_with("addons/") {
            counts.addons += 1;
        } else if name.starts_with("wtf/common/") {
            counts.wtf_common += 1;
        } else if name.starts_with("wtf/characters/") {
            counts.wtf_characters += 1;
        } else if name.starts_with("fonts/") {
            counts.fonts += 1;
        } else if name.starts_with("interface/") {
            counts.interface_assets += 1;
        }
    }

    Ok(counts)
}
