use std::path::PathBuf;

use super::super::map_owned_vec;
use super::{
    RuntimeDefaultableRequest, apply_backup_dir_default, apply_backup_output_default,
    resolve_optional_app_input_path, resolve_optional_app_output_path,
};
use crate::core::app::{AppRuntime, BackupGroupValue, ResolvedInstallationValue};
use crate::core::backup::{
    BackupRequest as DomainBackupRequest, RestoreBackupRequest as DomainRestoreBackupRequest,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone)]
pub struct ListBackupsRequest {
    pub backup_dir: Option<PathBuf>,
}

impl RuntimeDefaultableRequest for ListBackupsRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_dir_default(runtime, &mut self.backup_dir);
        self
    }
}

impl ListBackupsRequest {
    pub(crate) fn into_backup_dir(self, runtime: &AppRuntime) -> AppResult<Option<PathBuf>> {
        resolve_optional_backup_path(
            runtime,
            self.apply_runtime_defaults(runtime).backup_dir,
            "backup directory",
        )
    }
}

#[derive(Debug, Clone)]
pub struct CreateBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub output_path: Option<PathBuf>,
    pub groups: Vec<BackupGroupValue>,
    pub label: Option<String>,
}

impl RuntimeDefaultableRequest for CreateBackupAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.output_path);
        self
    }
}

impl CreateBackupAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainBackupRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            Ok(DomainBackupRequest {
                installation: request.installation.into_domain()?,
                output_path: resolve_optional_app_output_path(
                    runtime,
                    request.output_path,
                    "backup output directory",
                )?,
                groups: map_owned_vec(request.groups, BackupGroupValue::into_domain),
                label: request.label,
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct RestoreBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub archive_path: Option<PathBuf>,
    pub backup_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
}

impl RuntimeDefaultableRequest for RestoreBackupAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_dir_default(runtime, &mut self.backup_dir);
        self
    }
}

impl RestoreBackupAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainRestoreBackupRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            Ok(DomainRestoreBackupRequest {
                installation: request.installation.into_domain()?,
                archive_path: resolve_optional_backup_path(
                    runtime,
                    request.archive_path,
                    "backup archive",
                )?,
                backup_id: request.backup_id,
                backup_dir: resolve_optional_backup_path(
                    runtime,
                    request.backup_dir,
                    "backup directory",
                )?,
            })
        })
    }
}

fn resolve_optional_backup_path(
    runtime: &AppRuntime,
    path: Option<PathBuf>,
    description: &str,
) -> AppResult<Option<PathBuf>> {
    resolve_optional_app_input_path(runtime, path, description)
}
