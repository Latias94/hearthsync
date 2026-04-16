use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::model::{BackupGroup, BackupMetadata, BackupRequest, CreatedBackup, RestoredBackup};
use super::storage::resolve_backup_dir;
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

    fs::create_dir_all(&output_dir)?;

    let file_name = build_backup_file_name(
        request.installation.flavor.as_str(),
        request.label.as_deref(),
        &timestamp,
    );
    let archive_path = output_dir.join(file_name);
    let file = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archived_files = 0usize;

    for group in &request.groups {
        match group {
            BackupGroup::Addons => {
                archived_files += add_directory_group(
                    &mut zip,
                    &request.installation.addon_dir,
                    Path::new("addons"),
                )?;
            }
            BackupGroup::Wtf => {
                archived_files +=
                    add_directory_group(&mut zip, &request.installation.wtf_dir, Path::new("wtf"))?;
            }
            BackupGroup::Fonts => {
                archived_files += add_directory_group(
                    &mut zip,
                    &request.installation.fonts_dir,
                    Path::new("fonts"),
                )?;
            }
            BackupGroup::InterfaceAssets => {
                archived_files +=
                    add_interface_assets_group(&mut zip, &request.installation.interface_dir)?;
            }
        }
    }

    let metadata = BackupMetadata {
        schema_version: 1,
        created_at: timestamp,
        label: request
            .label
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        flavor: request.installation.flavor.as_str().to_string(),
        flavor_root: request.installation.flavor_root.clone(),
        groups: request.groups,
    };

    zip.start_file("backup.toml", zip_file_options())?;
    zip.write_all(toml::to_string_pretty(&metadata)?.as_bytes())?;
    zip.finish()?;

    Ok(CreatedBackup {
        archive_path,
        archived_files,
        metadata,
    })
}

pub fn restore_backup(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<RestoredBackup> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let metadata = read_backup_metadata(&mut archive)?;

    for group in &metadata.groups {
        clear_group_destination(*group, installation)?;
    }

    let mut restored_files = 0usize;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        if entry_name == "backup.toml" {
            continue;
        }

        let Some(destination) = map_backup_entry_to_destination(&entry_name, installation)? else {
            continue;
        };

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut output = File::create(destination)?;
        std::io::copy(&mut entry, &mut output)?;
        restored_files += 1;
    }

    Ok(RestoredBackup {
        archive_path: archive_path.to_path_buf(),
        restored_files,
        metadata,
    })
}

pub(super) fn read_backup_metadata_from_path(path: &Path) -> AppResult<BackupMetadata> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    read_backup_metadata(&mut archive)
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

fn add_directory_group(
    zip: &mut ZipWriter<File>,
    source_dir: &Path,
    archive_root: &Path,
) -> AppResult<usize> {
    if !source_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in WalkDir::new(source_dir).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source_dir)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let archive_path = archive_root.join(relative);

        if entry.file_type().is_dir() {
            zip.add_directory(to_zip_path(&archive_path), zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &archive_path)?;
        archived_files += 1;
    }

    Ok(archived_files)
}

fn add_interface_assets_group(zip: &mut ZipWriter<File>, interface_dir: &Path) -> AppResult<usize> {
    if !interface_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in fs::read_dir(interface_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().eq_ignore_ascii_case("AddOns") {
            continue;
        }

        let archive_root = Path::new("interface").join(name);
        if path.is_dir() {
            archived_files += add_directory_group(zip, &path, &archive_root)?;
        } else if path.is_file() {
            write_file_to_zip(zip, &path, &archive_root)?;
            archived_files += 1;
        }
    }

    Ok(archived_files)
}

fn write_file_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
) -> AppResult<()> {
    let mut file = File::open(source_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    zip.start_file(to_zip_path(archive_path), zip_file_options())?;
    zip.write_all(&buffer)?;
    Ok(())
}

fn to_zip_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn read_backup_metadata(archive: &mut ZipArchive<File>) -> AppResult<BackupMetadata> {
    let mut entry = archive.by_name("backup.toml")?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    Ok(toml::from_str(&content)?)
}

fn clear_group_destination(
    group: BackupGroup,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    match group {
        BackupGroup::Addons => clear_directory(&installation.addon_dir),
        BackupGroup::Wtf => clear_directory(&installation.wtf_dir),
        BackupGroup::Fonts => clear_directory(&installation.fonts_dir),
        BackupGroup::InterfaceAssets => clear_interface_assets(&installation.interface_dir),
    }
}

fn clear_directory(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn clear_interface_assets(interface_dir: &Path) -> AppResult<()> {
    if !interface_dir.exists() {
        fs::create_dir_all(interface_dir)?;
        return Ok(());
    }

    for entry in fs::read_dir(interface_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case("AddOns") {
            continue;
        }

        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn map_backup_entry_to_destination(
    entry_name: &str,
    installation: &DetectedFlavorInstallation,
) -> AppResult<Option<PathBuf>> {
    let segments = safe_zip_segments(entry_name)?;
    if segments.is_empty() {
        return Ok(None);
    }

    match segments.as_slice() {
        ["addons", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.addon_dir, rest)))
        }
        ["wtf", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.wtf_dir, rest)))
        }
        ["fonts", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.fonts_dir, rest)))
        }
        ["interface", rest @ ..] if !rest.is_empty() => {
            Ok(Some(join_segments(&installation.interface_dir, rest)))
        }
        _ => Ok(None),
    }
}

fn safe_zip_segments(entry_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in entry_name.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe backup path: `{entry_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
