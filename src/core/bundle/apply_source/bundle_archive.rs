use std::fs::File;
use std::path::Path;

use zip::ZipArchive;

use super::super::archive_read::{
    collect_bundle_entry_names, extract_archive_entry_to_path,
    read_bundle_entry_bytes_from_archive, read_manifest_from_archive,
};
use super::super::*;
use super::reader::ApplySourceReader;

pub(in crate::core::bundle::apply_source) fn bundle_manifest_from_archive(
    bundle_path: &Path,
) -> AppResult<BundleManifest> {
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    read_manifest_from_archive(&mut archive)
}

pub(in crate::core::bundle::apply_source) fn logical_entry_names_from_bundle_archive(
    bundle_path: &Path,
) -> AppResult<Vec<String>> {
    collect_bundle_entry_names(bundle_path)
}

pub(in crate::core::bundle::apply_source) fn open_bundle_archive_reader(
    bundle_path: &Path,
) -> AppResult<ApplySourceReader> {
    let file = File::open(bundle_path)?;
    Ok(ApplySourceReader::BundleArchive(ZipArchive::new(file)?))
}

pub(in crate::core::bundle::apply_source) fn read_bundle_archive_entry_bytes(
    reader: &mut ApplySourceReader,
    logical_name: &str,
) -> AppResult<Vec<u8>> {
    let archive = expect_bundle_archive_reader(reader)?;
    read_bundle_entry_bytes_from_archive(archive, logical_name)
}

pub(in crate::core::bundle::apply_source) fn materialize_bundle_archive_entry(
    reader: &mut ApplySourceReader,
    logical_name: &str,
    destination: &Path,
) -> AppResult<()> {
    let archive = expect_bundle_archive_reader(reader)?;
    extract_archive_entry_to_path(archive, logical_name, destination)
}

fn expect_bundle_archive_reader(
    reader: &mut ApplySourceReader,
) -> AppResult<&mut ZipArchive<File>> {
    match reader {
        ApplySourceReader::BundleArchive(archive) => Ok(archive),
        _ => Err(AppError::Validation(
            "bundle apply source expected a bundle archive reader".to_string(),
        )),
    }
}
