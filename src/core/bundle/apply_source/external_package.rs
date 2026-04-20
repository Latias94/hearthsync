use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::super::archive_read::entries::{
    extract_archive_entry_to_path, read_bundle_entry_bytes_from_archive,
};
use super::super::external_package::ExternalPackageSourceKind;
use super::super::shared::resolve_zip_style_path;
use super::reader::ApplySourceReader;
use crate::core::error::{AppError, AppResult};

pub(in crate::core::bundle::apply_source) fn logical_entry_names_from_external_package(
    entry_source_map: &BTreeMap<String, String>,
) -> AppResult<Vec<String>> {
    Ok(entry_source_map.keys().cloned().collect())
}

pub(in crate::core::bundle::apply_source) fn open_external_package_reader(
    source_path: &Path,
    source_kind: ExternalPackageSourceKind,
) -> AppResult<ApplySourceReader> {
    match source_kind {
        ExternalPackageSourceKind::ZipArchive => {
            let file = File::open(source_path)?;
            Ok(ApplySourceReader::ExternalPackageArchive(ZipArchive::new(
                file,
            )?))
        }
        ExternalPackageSourceKind::Directory => Ok(ApplySourceReader::ExternalPackageDirectory),
    }
}

pub(in crate::core::bundle::apply_source) fn read_external_package_entry_bytes(
    source_path: &Path,
    source_kind: ExternalPackageSourceKind,
    entry_source_map: &BTreeMap<String, String>,
    reader: &mut ApplySourceReader,
    logical_name: &str,
) -> AppResult<Vec<u8>> {
    match source_kind {
        ExternalPackageSourceKind::Directory => {
            let entry_path = resolve_external_package_source_entry_path(
                source_path,
                entry_source_map,
                logical_name,
            )?;
            fs::read(entry_path).map_err(Into::into)
        }
        ExternalPackageSourceKind::ZipArchive => {
            let source_entry_name =
                lookup_external_package_entry_source_path(entry_source_map, logical_name)?;
            let archive = expect_external_package_archive_reader(reader)?;
            read_bundle_entry_bytes_from_archive(archive, source_entry_name)
        }
    }
}

pub(in crate::core::bundle::apply_source) fn materialize_external_package_entry(
    source_path: &Path,
    source_kind: ExternalPackageSourceKind,
    entry_source_map: &BTreeMap<String, String>,
    reader: &mut ApplySourceReader,
    logical_name: &str,
    destination: &Path,
) -> AppResult<()> {
    match source_kind {
        ExternalPackageSourceKind::Directory => {
            let entry_path = resolve_external_package_source_entry_path(
                source_path,
                entry_source_map,
                logical_name,
            )?;
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry_path, destination)?;
            Ok(())
        }
        ExternalPackageSourceKind::ZipArchive => {
            let source_entry_name =
                lookup_external_package_entry_source_path(entry_source_map, logical_name)?;
            let archive = expect_external_package_archive_reader(reader)?;
            extract_archive_entry_to_path(archive, source_entry_name, destination)
        }
    }
}

fn expect_external_package_archive_reader(
    reader: &mut ApplySourceReader,
) -> AppResult<&mut ZipArchive<File>> {
    match reader {
        ApplySourceReader::ExternalPackageArchive(archive) => Ok(archive),
        _ => Err(AppError::Validation(
            "external package apply source expected an archive reader".to_string(),
        )),
    }
}

fn lookup_external_package_entry_source_path<'a>(
    entry_source_map: &'a BTreeMap<String, String>,
    logical_name: &str,
) -> AppResult<&'a str> {
    entry_source_map
        .get(logical_name)
        .map(String::as_str)
        .ok_or_else(|| {
            AppError::Validation(format!(
                "external-package apply operation is missing a source path: {logical_name}"
            ))
        })
}

fn resolve_external_package_source_entry_path(
    source_path: &Path,
    entry_source_map: &BTreeMap<String, String>,
    logical_name: &str,
) -> AppResult<PathBuf> {
    let source_entry_name =
        lookup_external_package_entry_source_path(entry_source_map, logical_name)?;
    resolve_zip_style_path(source_path, source_entry_name)
}
