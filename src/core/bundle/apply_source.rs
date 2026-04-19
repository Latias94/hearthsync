use std::fs::{self, File};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::archive_read::{
    collect_bundle_entry_names, extract_archive_entry_to_path,
    read_bundle_entry_bytes_from_archive, read_manifest_from_archive,
};
use super::*;

pub(super) enum ApplySourceReader {
    BundleArchive(ZipArchive<File>),
    ExternalPackageArchive(ZipArchive<File>),
    ExternalPackageDirectory,
}

impl PreparedApplySource {
    pub(super) fn bundle_manifest(&self) -> AppResult<BundleManifest> {
        match self {
            PreparedApplySource::BundleArchive { bundle_path } => {
                let file = File::open(bundle_path)?;
                let mut archive = ZipArchive::new(file)?;
                read_manifest_from_archive(&mut archive)
            }
            PreparedApplySource::ExternalPackage { .. } => Err(AppError::Validation(
                "only bundle archives contain embedded bundle manifests".to_string(),
            )),
        }
    }

    pub(super) fn logical_entry_names(&self) -> AppResult<Vec<String>> {
        match self {
            PreparedApplySource::BundleArchive { bundle_path } => {
                collect_bundle_entry_names(bundle_path)
            }
            PreparedApplySource::ExternalPackage {
                entry_source_map, ..
            } => Ok(entry_source_map.keys().cloned().collect()),
        }
    }

    pub(super) fn open_reader(&self) -> AppResult<ApplySourceReader> {
        match self {
            PreparedApplySource::BundleArchive { bundle_path } => {
                let file = File::open(bundle_path)?;
                Ok(ApplySourceReader::BundleArchive(ZipArchive::new(file)?))
            }
            PreparedApplySource::ExternalPackage {
                source_path,
                source_kind: ExternalPackageSourceKind::ZipArchive,
                ..
            } => {
                let file = File::open(source_path)?;
                Ok(ApplySourceReader::ExternalPackageArchive(ZipArchive::new(
                    file,
                )?))
            }
            PreparedApplySource::ExternalPackage {
                source_kind: ExternalPackageSourceKind::Directory,
                ..
            } => Ok(ApplySourceReader::ExternalPackageDirectory),
        }
    }

    pub(super) fn read_logical_entry_bytes(
        &self,
        reader: &mut ApplySourceReader,
        logical_name: &str,
    ) -> AppResult<Vec<u8>> {
        match self {
            PreparedApplySource::BundleArchive { .. } => {
                let archive = expect_bundle_archive_reader(reader)?;
                read_bundle_entry_bytes_from_archive(archive, logical_name)
            }
            PreparedApplySource::ExternalPackage {
                source_path,
                source_kind: ExternalPackageSourceKind::Directory,
                entry_source_map,
            } => {
                let entry_path = resolve_external_package_source_entry_path(
                    source_path,
                    entry_source_map,
                    logical_name,
                )?;
                fs::read(entry_path).map_err(Into::into)
            }
            PreparedApplySource::ExternalPackage {
                entry_source_map, ..
            } => {
                let source_entry_name =
                    lookup_external_package_entry_source_path(entry_source_map, logical_name)?;
                let archive = expect_external_package_archive_reader(reader)?;
                read_bundle_entry_bytes_from_archive(archive, source_entry_name)
            }
        }
    }

    pub(super) fn materialize_logical_entry(
        &self,
        reader: &mut ApplySourceReader,
        logical_name: &str,
        destination: &Path,
    ) -> AppResult<()> {
        match self {
            PreparedApplySource::BundleArchive { .. } => {
                let archive = expect_bundle_archive_reader(reader)?;
                extract_archive_entry_to_path(archive, logical_name, destination)
            }
            PreparedApplySource::ExternalPackage {
                source_path,
                source_kind: ExternalPackageSourceKind::Directory,
                entry_source_map,
            } => {
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
            PreparedApplySource::ExternalPackage {
                entry_source_map, ..
            } => {
                let source_entry_name =
                    lookup_external_package_entry_source_path(entry_source_map, logical_name)?;
                let archive = expect_external_package_archive_reader(reader)?;
                extract_archive_entry_to_path(archive, source_entry_name, destination)
            }
        }
    }
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
    entry_source_map: &'a std::collections::BTreeMap<String, String>,
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
    entry_source_map: &std::collections::BTreeMap<String, String>,
    logical_name: &str,
) -> AppResult<PathBuf> {
    let source_entry_name =
        lookup_external_package_entry_source_path(entry_source_map, logical_name)?;
    resolve_zip_style_path(source_path, source_entry_name)
}
