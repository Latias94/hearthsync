use std::path::PathBuf;

use super::{
    RuntimeDefaultableRequest, apply_backup_output_default, resolve_optional_app_output_path,
};
use crate::core::addon::index::{
    AddonIndexAttachRequest as DomainAddonIndexAttachRequest,
    AddonIndexInstallRequest as DomainAddonIndexInstallRequest,
    AddonIndexRelinkRequest as DomainAddonIndexRelinkRequest,
    AddonIndexScaffoldRequest as DomainAddonIndexScaffoldRequest,
    AddonIndexSuggestionRequest as DomainAddonIndexSuggestionRequest,
    AddonIndexUpdateRequest as DomainAddonIndexUpdateRequest,
};
use crate::core::app::{AppRuntime, ResolvedInstallationValue};
use crate::core::error::AppResult;

#[derive(Debug, Clone)]
pub struct InspectAddonIndexRequest {
    pub index_path: PathBuf,
}

impl InspectAddonIndexRequest {
    pub(crate) fn into_index_path(self, runtime: &AppRuntime) -> AppResult<PathBuf> {
        resolve_addon_index_path(runtime, self.index_path)
    }
}

#[derive(Debug, Clone)]
pub struct AttachAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub apply_ready_only: bool,
}

impl AttachAddonIndexAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAddonIndexAttachRequest> {
        let installation = self.installation.into_domain()?;
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainAddonIndexAttachRequest {
            installation,
            state_paths,
            index_path: resolve_addon_index_path(runtime, self.index_path)?,
            name: self.name,
            dry_run: self.dry_run,
            apply_ready_only: self.apply_ready_only,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SuggestAddonIndexRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: Option<String>,
}

impl SuggestAddonIndexRequest {
    pub(crate) fn into_domain(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAddonIndexSuggestionRequest> {
        let installation = self.installation.into_domain()?;
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainAddonIndexSuggestionRequest {
            installation,
            state_paths,
            index_path: resolve_addon_index_path(runtime, self.index_path)?,
            name: self.name,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ScaffoldAddonIndexRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub index_name: String,
    pub description: Option<String>,
    pub name: Option<String>,
    pub overwrite: bool,
}

impl ScaffoldAddonIndexRequest {
    pub(crate) fn into_domain(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAddonIndexScaffoldRequest> {
        let installation = self.installation.into_domain()?;
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainAddonIndexScaffoldRequest {
            installation,
            state_paths,
            index_path: resolve_addon_index_path(runtime, self.index_path)?,
            index_name: self.index_name,
            description: self.description,
            name: self.name,
            overwrite: self.overwrite,
        })
    }
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
    ) -> AppResult<DomainAddonIndexInstallRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            let installation = request.installation.into_domain()?;
            let state_paths = runtime.addon_state_paths(&installation)?;

            Ok(DomainAddonIndexInstallRequest {
                installation,
                state_paths,
                index_path: resolve_addon_index_path(runtime, request.index_path)?,
                name: request.name,
                dry_run: request.dry_run,
                backup_output_path: resolve_addon_index_backup_output_path(
                    runtime,
                    request.backup_output_path,
                )?,
                replace_existing: request.replace_existing,
            })
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
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAddonIndexUpdateRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            let installation = request.installation.into_domain()?;
            let state_paths = runtime.addon_state_paths(&installation)?;

            Ok(DomainAddonIndexUpdateRequest {
                installation,
                state_paths,
                index_path: resolve_addon_index_path(runtime, request.index_path)?,
                name: request.name,
                dry_run: request.dry_run,
                backup_output_path: resolve_addon_index_backup_output_path(
                    runtime,
                    request.backup_output_path,
                )?,
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct RelinkAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: String,
    pub target: Option<String>,
    pub dry_run: bool,
}

impl RelinkAddonIndexAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAddonIndexRelinkRequest> {
        let installation = self.installation.into_domain()?;
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainAddonIndexRelinkRequest {
            installation,
            state_paths,
            index_path: resolve_addon_index_path(runtime, self.index_path)?,
            name: self.name,
            target: self.target,
            dry_run: self.dry_run,
        })
    }
}

fn resolve_addon_index_path(runtime: &AppRuntime, path: PathBuf) -> AppResult<PathBuf> {
    runtime.resolve_input_path(path, "addon index file")
}

fn resolve_addon_index_backup_output_path(
    runtime: &AppRuntime,
    path: Option<PathBuf>,
) -> AppResult<Option<PathBuf>> {
    resolve_optional_app_output_path(runtime, path, "addon index backup output directory")
}
