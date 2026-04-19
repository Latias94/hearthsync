use std::path::PathBuf;

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

impl InstallAddonIndexAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainAddonIndexInstallRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainAddonIndexInstallRequest {
            installation: request.installation.into(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
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

impl UpdateAddonIndexAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainAddonIndexUpdateRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainAddonIndexUpdateRequest {
            installation: request.installation.into(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}
