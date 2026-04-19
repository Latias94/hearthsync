use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::BackupGroupValue;
use crate::core::backup::{
    BackupCatalog, BackupCatalogEntry, BackupMetadata, CreatedBackup as DomainCreatedBackup,
    RestoredBackup as DomainRestoredBackup,
};

#[derive(Debug, Clone, Serialize)]
pub struct BackupEntryResult {
    pub backup_id: String,
    pub archive_path: PathBuf,
    pub archive_size_bytes: u64,
    pub created_at: String,
    pub label: Option<String>,
    pub flavor: String,
    pub flavor_root: PathBuf,
    pub groups: Vec<BackupGroupValue>,
}

impl BackupEntryResult {
    pub(crate) fn from_domain(value: BackupCatalogEntry) -> Self {
        Self {
            backup_id: value.backup_id,
            archive_path: value.archive_path,
            archive_size_bytes: value.archive_size_bytes,
            created_at: value.metadata.created_at,
            label: value.metadata.label,
            flavor: value.metadata.flavor,
            flavor_root: value.metadata.flavor_root,
            groups: value
                .metadata
                .groups
                .into_iter()
                .map(BackupGroupValue::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupMetadataResult {
    pub schema_version: u32,
    pub created_at: String,
    pub label: Option<String>,
    pub flavor: String,
    pub flavor_root: PathBuf,
    pub group_count: usize,
    pub groups: Vec<BackupGroupValue>,
}

impl BackupMetadataResult {
    pub(crate) fn from_domain(value: BackupMetadata) -> Self {
        let group_count = value.groups.len();

        Self {
            schema_version: value.schema_version,
            created_at: value.created_at,
            label: value.label,
            flavor: value.flavor,
            flavor_root: value.flavor_root,
            group_count,
            groups: value
                .groups
                .into_iter()
                .map(BackupGroupValue::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBackupResult {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub metadata: BackupMetadataResult,
}

impl CreatedBackupResult {
    pub(crate) fn from_domain(value: DomainCreatedBackup) -> Self {
        Self {
            archive_path: value.archive_path,
            archived_files: value.archived_files,
            metadata: BackupMetadataResult::from_domain(value.metadata),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoredBackupResult {
    pub archive_path: PathBuf,
    pub restored_files: usize,
    pub metadata: BackupMetadataResult,
}

impl RestoredBackupResult {
    pub(crate) fn from_domain(value: DomainRestoredBackup) -> Self {
        Self {
            archive_path: value.archive_path,
            restored_files: value.restored_files,
            metadata: BackupMetadataResult::from_domain(value.metadata),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupCatalogResult {
    pub backup_dir: PathBuf,
    pub entry_count: usize,
    pub entries: Vec<BackupEntryResult>,
}

impl BackupCatalogResult {
    pub(crate) fn from_domain(value: BackupCatalog) -> Self {
        let entry_count = value.entries.len();

        Self {
            backup_dir: value.backup_dir,
            entry_count,
            entries: value
                .entries
                .into_iter()
                .map(BackupEntryResult::from_domain)
                .collect(),
        }
    }
}
