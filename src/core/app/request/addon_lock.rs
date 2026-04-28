use std::path::PathBuf;

use super::{
    RuntimeDefaultableRequest, apply_backup_output_default, resolve_app_input_path,
    resolve_optional_app_input_path, resolve_optional_app_output_path,
};
use crate::core::addon::lock::{
    AddonLockApplyRequest as DomainAddonLockApplyRequest,
    AddonLockSourceOverride as DomainAddonLockSourceOverride,
};
use crate::core::app::{AppRuntime, ResolvedInstallationValue};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone)]
pub struct InspectAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

impl InspectAddonLockRequest {
    pub(crate) fn into_domain_inputs(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<(
        DetectedFlavorInstallation,
        crate::core::addon::AddonStatePaths,
    )> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;
        Ok((installation, state_paths))
    }
}

#[derive(Debug, Clone)]
pub struct WriteAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

impl WriteAddonLockRequest {
    pub(crate) fn into_domain_inputs(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<(
        DetectedFlavorInstallation,
        crate::core::addon::AddonStatePaths,
    )> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;
        Ok((installation, state_paths))
    }
}

#[derive(Debug, Clone)]
pub struct DiffAddonLockRequest {
    pub left_lock_path: PathBuf,
    pub right_lock_path: PathBuf,
}

impl DiffAddonLockRequest {
    pub(crate) fn into_lock_paths(self, runtime: &AppRuntime) -> AppResult<(PathBuf, PathBuf)> {
        Ok((
            resolve_addon_lock_path(runtime, self.left_lock_path, "left addon lock file")?,
            resolve_addon_lock_path(runtime, self.right_lock_path, "right addon lock file")?,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct VerifyAddonLockRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
}

impl VerifyAddonLockRequest {
    pub(crate) fn into_domain_inputs(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<(
        DetectedFlavorInstallation,
        crate::core::addon::AddonStatePaths,
        Option<PathBuf>,
    )> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;
        Ok((
            installation,
            state_paths,
            resolve_optional_addon_lock_path(runtime, self.lock_path, "addon lock file")?,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct PlanAddonLockSyncRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
}

impl PlanAddonLockSyncRequest {
    pub(crate) fn into_domain_inputs(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<(
        DetectedFlavorInstallation,
        crate::core::addon::AddonStatePaths,
        Option<PathBuf>,
    )> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;
        Ok((
            installation,
            state_paths,
            resolve_optional_addon_lock_path(runtime, self.lock_path, "addon lock file")?,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct AddonLockSourceOverrideRequest {
    pub comparison_key: String,
    pub archive_path: PathBuf,
}

impl AddonLockSourceOverrideRequest {
    fn into_domain_override(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAddonLockSourceOverride> {
        Ok(DomainAddonLockSourceOverride {
            comparison_key: self.comparison_key,
            archive_path: resolve_addon_lock_path(
                runtime,
                self.archive_path,
                "addon lock source override archive",
            )?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ApplyAddonLockAppRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub source_overrides: Vec<AddonLockSourceOverrideRequest>,
}

impl RuntimeDefaultableRequest for ApplyAddonLockAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl ApplyAddonLockAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAddonLockApplyRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            let installation = request.installation.into_domain();
            let state_paths = runtime.addon_state_paths(&installation)?;

            Ok(DomainAddonLockApplyRequest {
                installation,
                state_paths,
                lock_path: resolve_optional_addon_lock_path(
                    runtime,
                    request.lock_path,
                    "addon lock file",
                )?,
                backup_output_path: resolve_optional_app_output_path(
                    runtime,
                    request.backup_output_path,
                    "addon lock backup output directory",
                )?,
                replace_existing: request.replace_existing,
                source_overrides: request
                    .source_overrides
                    .into_iter()
                    .map(|source_override| source_override.into_domain_override(runtime))
                    .collect::<AppResult<Vec<_>>>()?,
            })
        })
    }
}

fn resolve_addon_lock_path(
    runtime: &AppRuntime,
    path: PathBuf,
    description: &str,
) -> AppResult<PathBuf> {
    resolve_app_input_path(runtime, path, description)
}

fn resolve_optional_addon_lock_path(
    runtime: &AppRuntime,
    path: Option<PathBuf>,
    description: &str,
) -> AppResult<Option<PathBuf>> {
    resolve_optional_app_input_path(runtime, path, description)
}
