use std::path::PathBuf;

use super::super::map_owned_vec;
use super::{RuntimeDefaultableRequest, apply_backup_output_default};
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
        Ok((installation, state_paths, self.lock_path))
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
        Ok((installation, state_paths, self.lock_path))
    }
}

#[derive(Debug, Clone)]
pub struct AddonLockSourceOverrideRequest {
    pub comparison_key: String,
    pub archive_path: PathBuf,
}

impl AddonLockSourceOverrideRequest {
    fn into_domain_override(self) -> DomainAddonLockSourceOverride {
        DomainAddonLockSourceOverride {
            comparison_key: self.comparison_key,
            archive_path: self.archive_path,
        }
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
                lock_path: request.lock_path,
                backup_output_path: request.backup_output_path,
                replace_existing: request.replace_existing,
                source_overrides: map_owned_vec(
                    request.source_overrides,
                    AddonLockSourceOverrideRequest::into_domain_override,
                ),
            })
        })
    }
}
