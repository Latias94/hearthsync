use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use super::super::model::BackupMetadata;
use crate::core::archive_io::validate_zip_archive_entry;
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::boundary_validation::is_rfc3339_timestamp_shape;
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, WowFlavor};

pub(in crate::core::backup) fn read_backup_metadata_from_path(
    path: &Path,
) -> AppResult<BackupMetadata> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    read_backup_metadata(&mut archive)
}
pub(super) fn read_backup_metadata(archive: &mut ZipArchive<File>) -> AppResult<BackupMetadata> {
    let mut entry = archive.by_name("backup.toml")?;
    validate_zip_archive_entry(
        "backup metadata entry",
        entry.name(),
        entry.is_symlink(),
        entry.is_dir(),
    )?;
    if entry.is_dir() {
        return Err(AppError::Validation(
            "backup metadata entry must be a file: backup.toml".to_string(),
        ));
    }
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    let metadata = toml::from_str::<BackupMetadata>(&content)?;
    validate_backup_metadata_shape(&metadata)?;
    Ok(metadata)
}

pub(super) fn validate_backup_metadata_for_installation(
    metadata: &BackupMetadata,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    validate_backup_metadata_shape(metadata)?;

    if !metadata
        .flavor
        .eq_ignore_ascii_case(installation.flavor.as_str())
    {
        return Err(AppError::Validation(format!(
            "backup flavor `{}` does not match target flavor `{}`",
            metadata.flavor,
            installation.flavor.as_str()
        )));
    }

    Ok(())
}

fn validate_backup_metadata_shape(metadata: &BackupMetadata) -> AppResult<()> {
    if metadata.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported backup schema version: {}",
            metadata.schema_version
        )));
    }

    if !is_rfc3339_timestamp_shape(&metadata.created_at) {
        return Err(AppError::Validation(
            "backup metadata created_at must be an RFC 3339 timestamp".to_string(),
        ));
    }

    if metadata
        .label
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(AppError::Validation(
            "backup metadata label must not be blank".to_string(),
        ));
    }
    if let Some(label) = metadata.label.as_deref() {
        validate_portable_path_segment(label, "backup label")?;
    }

    if !is_supported_backup_flavor(&metadata.flavor) {
        return Err(AppError::Validation(format!(
            "unsupported backup flavor: {}",
            metadata.flavor
        )));
    }

    if metadata.flavor_root.as_os_str().is_empty() {
        return Err(AppError::Validation(
            "backup metadata flavor_root must not be empty".to_string(),
        ));
    }

    if metadata.groups.is_empty() {
        return Err(AppError::Validation(
            "backup metadata must include at least one group".to_string(),
        ));
    }

    let mut groups = BTreeSet::new();
    for group in &metadata.groups {
        if !groups.insert(*group) {
            return Err(AppError::Validation(format!(
                "backup metadata declares duplicate group `{}`",
                group.as_str()
            )));
        }
    }

    Ok(())
}

fn is_supported_backup_flavor(value: &str) -> bool {
    [
        WowFlavor::Retail,
        WowFlavor::Classic,
        WowFlavor::ClassicEra,
        WowFlavor::Ptr,
        WowFlavor::Beta,
        WowFlavor::Xptr,
    ]
    .iter()
    .any(|flavor| value.eq_ignore_ascii_case(flavor.as_str()))
}
