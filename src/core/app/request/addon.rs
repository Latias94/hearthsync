use std::path::PathBuf;

use crate::core::addon::{
    InstallAddonRequest as DomainInstallAddonRequest,
    RemoveAddonRequest as DomainRemoveAddonRequest, SearchAddonRequest as DomainSearchAddonRequest,
    UpdateAddonRequest as DomainUpdateAddonRequest,
};
use crate::core::app::{AddonPackageMetadataValue, AppRuntime, ResolvedInstallationValue};
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone)]
pub struct SearchAddonsRequest {
    pub installation: ResolvedInstallationValue,
    pub query: String,
    pub limit: usize,
}

impl SearchAddonsRequest {
    pub(crate) fn into_domain_request(self) -> DomainSearchAddonRequest {
        DomainSearchAddonRequest {
            installation: self.installation.into(),
            query: self.query,
            limit: self.limit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListAddonsRequest {
    pub installation: ResolvedInstallationValue,
}

impl ListAddonsRequest {
    pub(crate) fn into_domain_installation(self) -> DetectedFlavorInstallation {
        self.installation.into()
    }
}

#[derive(Debug, Clone)]
pub struct InstallAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub source: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub metadata: Option<AddonPackageMetadataValue>,
}

impl InstallAddonAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainInstallAddonRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainInstallAddonRequest {
            installation: request.installation.into(),
            source: request.source,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            metadata: request.metadata.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl UpdateAddonAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainUpdateAddonRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainUpdateAddonRequest {
            installation: request.installation.into(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoveAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl RemoveAddonAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainRemoveAddonRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainRemoveAddonRequest {
            installation: request.installation.into(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}
