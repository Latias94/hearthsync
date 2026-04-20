use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use super::super::shared::path::safe_zip_segments;
use crate::core::archive_io::copy_reader_to_path;
use crate::core::error::{AppError, AppResult};

pub(in crate::core::bundle) fn collect_bundle_entry_names(
    bundle_path: &Path,
) -> AppResult<Vec<String>> {
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

pub(in crate::core::bundle) fn read_bundle_entry_bytes_from_archive(
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

pub(in crate::core::bundle) fn extract_archive_entry_to_path(
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
