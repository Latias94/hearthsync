use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;
use zip::ZipWriter;

use super::shared::path::{should_skip_path, to_zip_path};
use super::shared::zip_options::{zip_dir_options, zip_file_options};
use crate::core::archive_io::{
    PortableArchivePathSet, add_directory_to_zip, portable_archive_path_issue_error,
    start_file_to_zip, stream_file_to_zip,
};
use crate::core::error::{AppError, AppResult};

pub(super) fn add_path_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    if !source_path.exists() {
        return Ok(0);
    }

    if source_path.is_file() {
        write_file_to_zip(zip, source_path, archive_path, archive_outputs)?;
        return Ok(1);
    }

    let mut archived_files = 0usize;
    for entry in WalkDir::new(source_path).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source_path)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() || should_skip_path(relative) {
            continue;
        }

        let target_path = archive_path.join(relative);
        if entry.file_type().is_dir() {
            let archive_name = to_zip_path(&target_path);
            register_bundle_archive_output(archive_outputs, &archive_name, true)?;
            add_directory_to_zip(zip, &archive_name, zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &target_path, archive_outputs)?;
        archived_files += 1;
    }

    Ok(archived_files)
}

fn write_file_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<()> {
    let archive_name = to_zip_path(archive_path);
    register_bundle_archive_output(archive_outputs, &archive_name, false)?;
    stream_file_to_zip(zip, source_path, &archive_name, zip_file_options())
}

pub(super) fn write_toml_to_zip<T: Serialize>(
    zip: &mut ZipWriter<File>,
    archive_path: &str,
    value: &T,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    register_bundle_archive_output(archive_outputs, archive_path, false)?;
    start_file_to_zip(zip, archive_path, zip_file_options())?;
    zip.write_all(toml::to_string_pretty(value)?.as_bytes())?;
    Ok(1)
}

pub(super) fn register_bundle_archive_output(
    archive_outputs: &mut PortableArchivePathSet,
    archive_path: &str,
    is_directory: bool,
) -> AppResult<()> {
    archive_outputs
        .register(archive_path, is_directory)
        .map_err(|issue| portable_archive_path_issue_error("bundle creation", issue))
}

#[cfg(test)]
mod tests {
    use crate::core::archive_io::PortableArchivePathSet;

    use super::register_bundle_archive_output;

    #[test]
    fn register_bundle_archive_output_rejects_case_insensitive_metadata_collisions() {
        let mut archive_outputs = PortableArchivePathSet::new();
        register_bundle_archive_output(
            &mut archive_outputs,
            "metadata/addons/indexes/addon-index.toml",
            false,
        )
        .expect("first metadata entry should register");

        let error = register_bundle_archive_output(
            &mut archive_outputs,
            "metadata/addons/indexes/ADDON-INDEX.toml",
            false,
        )
        .expect_err("case-only metadata collision should fail");

        let message = error.to_string();
        assert!(message.contains("case-insensitive archive path collisions"));
        assert!(message.contains("metadata/addons/indexes/addon-index.toml"));
        assert!(message.contains("metadata/addons/indexes/ADDON-INDEX.toml"));
    }

    #[test]
    fn register_bundle_archive_output_rejects_file_as_ancestor_sidecar_conflicts() {
        let mut archive_outputs = PortableArchivePathSet::new();
        register_bundle_archive_output(&mut archive_outputs, "metadata/addons/sources", false)
            .expect("source root file should register");

        let error = register_bundle_archive_output(
            &mut archive_outputs,
            "metadata/addons/sources/addons-weakauras.zip",
            false,
        )
        .expect_err("file ancestor conflict should fail");

        let message = error.to_string();
        assert!(message.contains("conflicting file and directory archive paths"));
        assert!(message.contains("metadata/addons/sources"));
        assert!(message.contains("metadata/addons/sources/addons-weakauras.zip"));
    }

    #[test]
    fn register_bundle_archive_output_allows_directory_as_sidecar_ancestor() {
        let mut archive_outputs = PortableArchivePathSet::new();
        register_bundle_archive_output(&mut archive_outputs, "metadata/addons/sources", true)
            .expect("sidecar directory should register");
        register_bundle_archive_output(
            &mut archive_outputs,
            "metadata/addons/sources/addons-weakauras.zip",
            false,
        )
        .expect("directory ancestors should stay legal");
    }
}
