use std::path::PathBuf;

use super::{RuntimeDefaultableRequest, apply_backup_output_default};
use crate::core::addon::{
    AdoptAddonsRequest as DomainAdoptAddonsRequest,
    InstallAddonRequest as DomainInstallAddonRequest,
    RelinkAddonRequest as DomainRelinkAddonRequest, RemoveAddonRequest as DomainRemoveAddonRequest,
    SearchAddonRequest as DomainSearchAddonRequest, UpdateAddonRequest as DomainUpdateAddonRequest,
    addon_source_input_is_local_archive,
};
use crate::core::app::{AddonPackageMetadataValue, AppRuntime, ResolvedInstallationValue};
use crate::core::error::AppResult;
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
pub struct AdoptAddonsAppRequest {
    pub installation: ResolvedInstallationValue,
    pub addon_directories: Vec<String>,
    pub package_id: Option<String>,
    pub archive_output_path: Option<PathBuf>,
    pub dry_run: bool,
}

impl AdoptAddonsAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAdoptAddonsRequest> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainAdoptAddonsRequest {
            installation,
            state_paths,
            addon_directories: self.addon_directories,
            package_id: self.package_id,
            archive_output_path: self.archive_output_path,
            dry_run: self.dry_run,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RelinkAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: String,
    pub source: String,
    pub dry_run: bool,
}

impl RelinkAddonAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainRelinkAddonRequest> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainRelinkAddonRequest {
            installation,
            state_paths,
            name: self.name,
            source: resolve_addon_source_input(runtime, self.source)?,
            dry_run: self.dry_run,
        })
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
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainInstallAddonRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            let installation = request.installation.into_domain();
            let state_paths = runtime.addon_state_paths(&installation)?;

            Ok(DomainInstallAddonRequest {
                installation,
                state_paths,
                source: resolve_addon_source_input(runtime, request.source)?,
                dry_run: request.dry_run,
                backup_output_path: request.backup_output_path,
                replace_existing: request.replace_existing,
                metadata: request.metadata.map(AddonPackageMetadataValue::into_domain),
            })
        })
    }
}

fn resolve_addon_source_input(runtime: &AppRuntime, source: String) -> AppResult<String> {
    if !addon_source_input_is_local_archive(&source) {
        return Ok(source);
    }

    let path = PathBuf::from(&source);
    if path.is_absolute() {
        return Ok(source);
    }

    Ok(runtime
        .resolve_input_path(path, "addon local archive source")?
        .display()
        .to_string())
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
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainUpdateAddonRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            let installation = request.installation.into_domain();
            let state_paths = runtime.addon_state_paths(&installation)?;

            Ok(DomainUpdateAddonRequest {
                installation,
                state_paths,
                name: request.name,
                dry_run: request.dry_run,
                backup_output_path: request.backup_output_path,
            })
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
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainRemoveAddonRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            let installation = request.installation.into_domain();
            let state_paths = runtime.addon_state_paths(&installation)?;

            Ok(DomainRemoveAddonRequest {
                installation,
                state_paths,
                name: request.name,
                dry_run: request.dry_run,
                backup_output_path: request.backup_output_path,
            })
        })
    }
}
