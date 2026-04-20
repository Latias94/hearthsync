use std::path::PathBuf;

use super::{RuntimeDefaultableRequest, apply_backup_output_default};
use crate::core::addon::lock::{
    AddonLockApplyRequest as DomainAddonLockApplyRequest,
    AddonLockSourceOverride as DomainAddonLockSourceOverride,
};
use crate::core::app::{AppRuntime, ResolvedInstallationValue};
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone)]
pub struct InspectAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

impl InspectAddonLockRequest {
    pub(crate) fn into_domain_installation(self) -> DetectedFlavorInstallation {
        self.installation.into_domain()
    }
}

#[derive(Debug, Clone)]
pub struct WriteAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

impl WriteAddonLockRequest {
    pub(crate) fn into_domain_installation(self) -> DetectedFlavorInstallation {
        self.installation.into_domain()
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
    pub(crate) fn into_domain_inputs(self) -> (DetectedFlavorInstallation, Option<PathBuf>) {
        (self.installation.into_domain(), self.lock_path)
    }
}

#[derive(Debug, Clone)]
pub struct PlanAddonLockSyncRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
}

impl PlanAddonLockSyncRequest {
    pub(crate) fn into_domain_inputs(self) -> (DetectedFlavorInstallation, Option<PathBuf>) {
        (self.installation.into_domain(), self.lock_path)
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
    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainAddonLockApplyRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| DomainAddonLockApplyRequest {
            installation: request.installation.into_domain(),
            lock_path: request.lock_path,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            source_overrides: request
                .source_overrides
                .into_iter()
                .map(AddonLockSourceOverrideRequest::into_domain_override)
                .collect(),
        })
    }
}
