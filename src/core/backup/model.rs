use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
