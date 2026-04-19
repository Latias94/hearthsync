use std::path::PathBuf;

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

impl ApplyAddonLockAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainAddonLockApplyRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainAddonLockApplyRequest {
            installation: request.installation.into_domain(),
            lock_path: request.lock_path,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            source_overrides: request
                .source_overrides
                .into_iter()
                .map(AddonLockSourceOverrideRequest::into_domain_override)
                .collect(),
        }
    }
}
