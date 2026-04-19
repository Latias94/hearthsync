use std::path::PathBuf;

use crate::core::app::{AppRuntime, BackupGroupValue, ResolvedInstallationValue};
use crate::core::backup::{
    BackupRequest as DomainBackupRequest, RestoreBackupRequest as DomainRestoreBackupRequest,
};

#[derive(Debug, Clone)]
pub struct ListBackupsRequest {
    pub backup_dir: Option<PathBuf>,
}

impl ListBackupsRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_dir = runtime.backup_dir_or_default(self.backup_dir);
        self
    }

    pub(crate) fn into_backup_dir(self, runtime: &AppRuntime) -> Option<PathBuf> {
        self.apply_runtime_defaults(runtime).backup_dir
    }
}

#[derive(Debug, Clone)]
pub struct CreateBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub output_path: Option<PathBuf>,
    pub groups: Vec<BackupGroupValue>,
    pub label: Option<String>,
}

impl CreateBackupAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.output_path = runtime.backup_output_or_default(self.output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainBackupRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainBackupRequest {
            installation: request.installation.into_domain(),
            output_path: request.output_path,
            groups: request
                .groups
                .into_iter()
                .map(BackupGroupValue::into_domain)
                .collect(),
            label: request.label,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RestoreBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub archive_path: Option<PathBuf>,
    pub backup_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
}

impl RestoreBackupAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_dir = runtime.backup_dir_or_default(self.backup_dir);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainRestoreBackupRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainRestoreBackupRequest {
            installation: request.installation.into_domain(),
            archive_path: request.archive_path,
            backup_id: request.backup_id,
            backup_dir: request.backup_dir,
        }
    }
}
