use std::path::Path;

use super::super::*;
use super::bundle_archive::{
    bundle_manifest_from_archive, logical_entry_names_from_bundle_archive,
    materialize_bundle_archive_entry, open_bundle_archive_reader, read_bundle_archive_entry_bytes,
};
use super::external_package::{
    logical_entry_names_from_external_package, materialize_external_package_entry,
    open_external_package_reader, read_external_package_entry_bytes,
};
use super::reader::ApplySourceReader;

impl PreparedApplySource {
    pub(in crate::core::bundle) fn bundle_manifest(&self) -> AppResult<BundleManifest> {
        match self {
            PreparedApplySource::BundleArchive { bundle_path } => {
                bundle_manifest_from_archive(bundle_path)
            }
            PreparedApplySource::ExternalPackage { .. } => Err(AppError::Validation(
                "only bundle archives contain embedded bundle manifests".to_string(),
            )),
        }
    }

    pub(in crate::core::bundle) fn logical_entry_names(&self) -> AppResult<Vec<String>> {
        match self {
            PreparedApplySource::BundleArchive { bundle_path } => {
                logical_entry_names_from_bundle_archive(bundle_path)
            }
            PreparedApplySource::ExternalPackage {
                entry_source_map, ..
            } => logical_entry_names_from_external_package(entry_source_map),
        }
    }

    pub(in crate::core::bundle) fn open_reader(&self) -> AppResult<ApplySourceReader> {
        match self {
            PreparedApplySource::BundleArchive { bundle_path } => {
                open_bundle_archive_reader(bundle_path)
            }
            PreparedApplySource::ExternalPackage {
                source_path,
                source_kind,
                ..
            } => open_external_package_reader(source_path, *source_kind),
        }
    }

    pub(in crate::core::bundle) fn read_logical_entry_bytes(
        &self,
        reader: &mut ApplySourceReader,
        logical_name: &str,
    ) -> AppResult<Vec<u8>> {
        match self {
            PreparedApplySource::BundleArchive { .. } => {
                read_bundle_archive_entry_bytes(reader, logical_name)
            }
            PreparedApplySource::ExternalPackage {
                source_path,
                source_kind,
                entry_source_map,
            } => read_external_package_entry_bytes(
                source_path,
                *source_kind,
                entry_source_map,
                reader,
                logical_name,
            ),
        }
    }

    pub(in crate::core::bundle) fn materialize_logical_entry(
        &self,
        reader: &mut ApplySourceReader,
        logical_name: &str,
        destination: &Path,
    ) -> AppResult<()> {
        match self {
            PreparedApplySource::BundleArchive { .. } => {
                materialize_bundle_archive_entry(reader, logical_name, destination)
            }
            PreparedApplySource::ExternalPackage {
                source_path,
                source_kind,
                entry_source_map,
            } => materialize_external_package_entry(
                source_path,
                *source_kind,
                entry_source_map,
                reader,
                logical_name,
                destination,
            ),
        }
    }
}
