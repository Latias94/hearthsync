use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::super::model::{
    BackupGroup, BackupMetadata, BackupRequest, CreatedBackup, normalize_backup_label,
};
use super::super::storage::resolve_backup_dir;
use super::metadata::validate_backup_metadata_for_installation;
use crate::core::archive_io::{
    PortableArchivePathSet, add_directory_to_zip, portable_archive_path_issue_error,
    reject_unsupported_symlink_metadata, start_file_to_zip, stream_file_to_zip,
};
use crate::core::archive_path::to_zip_path;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
pub fn create_backup(request: BackupRequest) -> AppResult<CreatedBackup> {
    if request.groups.is_empty() {
        return Err(AppError::Validation(
            "backup request must include at least one group".to_string(),
        ));
    }

    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let output_dir = resolve_backup_dir(request.output_path.as_deref())?;
    let label = normalize_backup_label(request.label)?;

    fs::create_dir_all(&output_dir)?;

    let file_name = build_backup_file_name(
        request.installation.flavor.as_str(),
        label.as_deref(),
        &timestamp,
    );
    let archive_path = output_dir.join(file_name);
    let file = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archive_outputs = PortableArchivePathSet::new();
    let mut archived_files = 0usize;

    for group in &request.groups {
        archived_files += add_backup_group_to_zip(
            &mut zip,
            &request.installation,
            *group,
            &mut archive_outputs,
        )?;
    }

    let metadata = BackupMetadata {
        schema_version: 1,
        created_at: timestamp,
        label,
        flavor: request.installation.flavor.as_str().to_string(),
        flavor_root: request.installation.flavor_root.clone(),
        groups: request.groups,
    };
    validate_backup_metadata_for_installation(&metadata, &request.installation)?;

    register_backup_archive_output(&mut archive_outputs, "backup.toml", false)?;
    start_file_to_zip(&mut zip, "backup.toml", zip_file_options())?;
    zip.write_all(toml::to_string_pretty(&metadata)?.as_bytes())?;
    zip.finish()?;

    Ok(CreatedBackup {
        archive_path,
        archived_files,
        metadata,
    })
}

fn build_backup_file_name(flavor: &str, label: Option<&str>, timestamp: &str) -> String {
    let compact_timestamp = timestamp
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect::<String>();

    match label {
        Some(value) if !value.trim().is_empty() => {
            format!("backup-{flavor}-{value}-{compact_timestamp}.zip")
        }
        _ => format!("backup-{flavor}-{compact_timestamp}.zip"),
    }
}

fn add_backup_group_to_zip(
    zip: &mut ZipWriter<File>,
    installation: &DetectedFlavorInstallation,
    group: BackupGroup,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    match group {
        BackupGroup::InterfaceAssets => {
            add_interface_assets_group(zip, group.installation_root(installation), archive_outputs)
        }
        _ => add_directory_group(
            zip,
            group.installation_root(installation),
            Path::new(group.archive_root_name()),
            archive_outputs,
        ),
    }
}

fn add_directory_group(
    zip: &mut ZipWriter<File>,
    source_dir: &Path,
    archive_root: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    if !source_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in WalkDir::new(source_dir).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let file_type = entry.file_type();
        let path = entry.path();
        reject_unsupported_backup_source_symlink("directory", path, file_type.is_symlink())?;
        let relative = path
            .strip_prefix(source_dir)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let archive_path = archive_root.join(relative);

        if file_type.is_dir() {
            let archive_name = to_zip_path(&archive_path);
            register_backup_archive_output(archive_outputs, &archive_name, true)?;
            add_directory_to_zip(zip, &archive_name, zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &archive_path, archive_outputs)?;
        archived_files += 1;
    }

    Ok(archived_files)
}

fn add_interface_assets_group(
    zip: &mut ZipWriter<File>,
    interface_dir: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    if !interface_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in fs::read_dir(interface_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        reject_unsupported_backup_source_symlink("interface asset", &path, file_type.is_symlink())?;
        let name = entry.file_name();
        if name.to_string_lossy().eq_ignore_ascii_case("AddOns") {
            continue;
        }

        let archive_root = Path::new("interface").join(name);
        if file_type.is_dir() {
            archived_files += add_directory_group(zip, &path, &archive_root, archive_outputs)?;
        } else if file_type.is_file() {
            write_file_to_zip(zip, &path, &archive_root, archive_outputs)?;
            archived_files += 1;
        }
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
    register_backup_archive_output(archive_outputs, &archive_name, false)?;
    stream_file_to_zip(zip, source_path, &archive_name, zip_file_options())
}

pub(super) fn register_backup_archive_output(
    archive_outputs: &mut PortableArchivePathSet,
    archive_path: &str,
    is_directory: bool,
) -> AppResult<()> {
    archive_outputs
        .register(archive_path, is_directory)
        .map_err(|issue| portable_archive_path_issue_error("backup creation", issue))
}

pub(in crate::core::backup) fn reject_unsupported_backup_source_symlink(
    source_kind: &str,
    entry_path: &Path,
    is_symlink: bool,
) -> AppResult<()> {
    reject_unsupported_symlink_metadata(
        &format!("backup {source_kind} entry"),
        &entry_path.display().to_string(),
        is_symlink,
    )
}

fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
