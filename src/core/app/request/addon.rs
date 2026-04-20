use std::path::PathBuf;

use super::{RuntimeDefaultableRequest, apply_backup_output_default};
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
            installation: self.installation.into_domain(),
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
        self.installation.into_domain()
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

impl RuntimeDefaultableRequest for InstallAddonAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl InstallAddonAppRequest {
    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainInstallAddonRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| DomainInstallAddonRequest {
            installation: request.installation.into_domain(),
            source: request.source,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            metadata: request.metadata.map(AddonPackageMetadataValue::into_domain),
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl RuntimeDefaultableRequest for UpdateAddonAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl UpdateAddonAppRequest {
    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainUpdateAddonRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| DomainUpdateAddonRequest {
            installation: request.installation.into_domain(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RemoveAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl RuntimeDefaultableRequest for RemoveAddonAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl RemoveAddonAppRequest {
    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainRemoveAddonRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| DomainRemoveAddonRequest {
            installation: request.installation.into_domain(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        })
    }
}
