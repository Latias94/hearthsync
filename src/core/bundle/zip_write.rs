use std::fs::File;
use std::io::Write;
use std::path::Path;

use serde::Serialize;
use walkdir::WalkDir;
use zip::ZipWriter;

use super::shared::path::{should_skip_path, to_zip_path};
use super::shared::zip_options::{zip_dir_options, zip_file_options};
use crate::core::archive_io::{
    PortableArchivePathIssue, PortableArchivePathIssueKind, PortableArchivePathSet,
    add_directory_to_zip, start_file_to_zip, stream_file_to_zip,
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
        .map_err(map_bundle_archive_output_issue)
}

fn map_bundle_archive_output_issue(issue: PortableArchivePathIssue) -> AppError {
    match issue.kind {
        PortableArchivePathIssueKind::ExactCollision => AppError::Validation(format!(
            "bundle creation would emit multiple archive entries onto the same path: `{}` and `{}`",
            issue.previous, issue.current
        )),
        PortableArchivePathIssueKind::CaseInsensitiveCollision => AppError::Validation(format!(
            "bundle creation would emit case-insensitive archive path collisions: `{}` and `{}` would map to the same path on Windows/default macOS targets",
            issue.previous, issue.current
        )),
        PortableArchivePathIssueKind::ExactPrefixConflict => AppError::Validation(format!(
            "bundle creation would emit conflicting file and directory archive paths: `{}` and `{}`",
            issue.previous, issue.current
        )),
        PortableArchivePathIssueKind::CaseInsensitivePrefixConflict => {
            AppError::Validation(format!(
                "bundle creation would emit case-insensitive file and directory archive path conflicts: `{}` and `{}` would create file/directory collisions on Windows/default macOS targets",
                issue.previous, issue.current
            ))
        }
    }
}
