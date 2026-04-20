use std::path::PathBuf;

use super::{RuntimeDefaultableRequest, apply_backup_output_default};
use crate::core::addon::index::{
    AddonIndexInstallRequest as DomainAddonIndexInstallRequest,
    AddonIndexUpdateRequest as DomainAddonIndexUpdateRequest,
};
use crate::core::app::{AppRuntime, ResolvedInstallationValue};

#[derive(Debug, Clone)]
pub struct InspectAddonIndexRequest {
    pub index_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InstallAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl RuntimeDefaultableRequest for InstallAddonIndexAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl InstallAddonIndexAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainAddonIndexInstallRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| DomainAddonIndexInstallRequest {
            installation: request.installation.into_domain(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl RuntimeDefaultableRequest for UpdateAddonIndexAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl UpdateAddonIndexAppRequest {
    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainAddonIndexUpdateRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| DomainAddonIndexUpdateRequest {
            installation: request.installation.into_domain(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        })
    }
}
