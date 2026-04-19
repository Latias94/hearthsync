use std::path::PathBuf;

use crate::core::app::{
    AppRuntime, BundleApplyMappingsValue, BundleManifestValue, ResolvedInstallationValue,
};
use crate::core::bundle::{
    BundleAddonLockApplyRequest as DomainBundleAddonLockApplyRequest,
    BundleApplyMappings as DomainBundleApplyMappings, PackBundleRequest as DomainPackBundleRequest,
    UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone)]
pub struct InspectBundleRequest {
    pub bundle_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PackBundleAppRequest {
    pub installation: ResolvedInstallationValue,
    pub manifest: BundleManifestValue,
    pub output_path: Option<PathBuf>,
    pub manifest_base_dir: Option<PathBuf>,
}

impl PackBundleAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.output_path = runtime.bundle_output_or_default(self.output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainPackBundleRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainPackBundleRequest {
            installation: request.installation.into(),
            manifest: request.manifest.into(),
            output_path: request.output_path,
            manifest_base_dir: request.manifest_base_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanBundleApplyRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl PlanBundleApplyRequest {
    pub(crate) fn into_domain_inputs(
        self,
    ) -> (
        PathBuf,
        DetectedFlavorInstallation,
        DomainBundleApplyMappings,
    ) {
        (
            self.bundle_path,
            self.installation.into(),
            self.apply_mappings.into(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct ApplyBundleAppRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl ApplyBundleAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainUnpackBundleRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainUnpackBundleRequest {
            bundle_path: request.bundle_path,
            installation: request.installation.into(),
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            apply_mappings: request.apply_mappings.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanBundleAddonLockRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
}

impl PlanBundleAddonLockRequest {
    pub(crate) fn into_domain_inputs(self) -> (PathBuf, DetectedFlavorInstallation) {
        (self.bundle_path, self.installation.into())
    }
}

#[derive(Debug, Clone)]
pub struct ApplyBundleAddonLockAppRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl ApplyBundleAddonLockAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainBundleAddonLockApplyRequest {
        let request = self.apply_runtime_defaults(runtime);

        DomainBundleAddonLockApplyRequest {
            bundle_path: request.bundle_path,
            installation: request.installation.into(),
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
    }
}
