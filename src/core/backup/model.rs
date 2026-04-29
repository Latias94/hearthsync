use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::archive_path::validate_portable_path_segment;
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupGroup {
    Addons,
    Wtf,
    Fonts,
    InterfaceAssets,
}

impl BackupGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Addons => "addons",
            Self::Wtf => "wtf",
            Self::Fonts => "fonts",
            Self::InterfaceAssets => "interface_assets",
        }
    }

    pub(crate) fn archive_root_name(&self) -> &'static str {
        match self {
            Self::Addons => "addons",
            Self::Wtf => "wtf",
            Self::Fonts => "fonts",
            Self::InterfaceAssets => "interface",
        }
    }

    pub(crate) fn from_archive_root_name(value: &str) -> Option<Self> {
        match value {
            "addons" => Some(Self::Addons),
            "wtf" => Some(Self::Wtf),
            "fonts" => Some(Self::Fonts),
            "interface" => Some(Self::InterfaceAssets),
            _ => None,
        }
    }

    pub(crate) fn installation_root<'a>(
        &self,
        installation: &'a DetectedFlavorInstallation,
    ) -> &'a Path {
        match self {
            Self::Addons => &installation.addon_dir,
            Self::Wtf => &installation.wtf_dir,
            Self::Fonts => &installation.fonts_dir,
            Self::InterfaceAssets => &installation.interface_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BackupRequest {
    pub installation: DetectedFlavorInstallation,
    pub output_path: Option<PathBuf>,
    pub groups: Vec<BackupGroup>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub schema_version: u32,
    pub created_at: String,
    #[serde(default)]
    pub label: Option<String>,
    pub flavor: String,
    pub flavor_root: PathBuf,
    pub groups: Vec<BackupGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBackup {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoredBackup {
    pub archive_path: PathBuf,
    pub restored_files: usize,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupCatalog {
    pub backup_dir: PathBuf,
    pub entries: Vec<BackupCatalogEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupCatalogEntry {
    pub backup_id: String,
    pub archive_path: PathBuf,
    pub archive_size_bytes: u64,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone)]
pub struct RestoreBackupRequest {
    pub installation: DetectedFlavorInstallation,
    pub archive_path: Option<PathBuf>,
    pub backup_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
}

pub(crate) fn normalize_backup_label(label: Option<String>) -> AppResult<Option<String>> {
    let Some(label) = label else {
        return Ok(None);
    };
    let label = label.trim().to_string();
    if label.is_empty() {
        return Ok(None);
    }

    validate_portable_path_segment(&label, "backup label")?;
    Ok(Some(label))
}

#[cfg(test)]
mod tests {
    use super::{BackupGroup, normalize_backup_label};

    #[test]
    fn backup_group_archive_root_name_distinguishes_metadata_label() {
        assert_eq!(BackupGroup::InterfaceAssets.as_str(), "interface_assets");
        assert_eq!(
            BackupGroup::InterfaceAssets.archive_root_name(),
            "interface"
        );
    }

    #[test]
    fn backup_group_archive_root_name_roundtrips() {
        for group in [
            BackupGroup::Addons,
            BackupGroup::Wtf,
            BackupGroup::Fonts,
            BackupGroup::InterfaceAssets,
        ] {
            assert_eq!(
                BackupGroup::from_archive_root_name(group.archive_root_name()),
                Some(group)
            );
        }

        assert_eq!(BackupGroup::from_archive_root_name("metadata"), None);
    }

    #[test]
    fn normalize_backup_label_trims_blank_and_rejects_non_portable_names() {
        assert_eq!(normalize_backup_label(None).expect("none label"), None);
        assert_eq!(
            normalize_backup_label(Some("  nightly ".to_string())).expect("trim label"),
            Some("nightly".to_string())
        );
        assert_eq!(
            normalize_backup_label(Some(" ".to_string())).expect("blank label"),
            None
        );

        let error = normalize_backup_label(Some("../escape".to_string()))
            .expect_err("path-shaped labels should fail");

        assert!(error.to_string().contains("invalid backup label name"));
    }
}
