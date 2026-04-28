use std::path::PathBuf;

use super::{
    RuntimeDefaultableRequest, apply_backup_output_default, apply_bundle_output_default,
    resolve_app_input_path, resolve_optional_app_input_path, resolve_optional_app_output_path,
};
use crate::core::app::{
    AppRuntime, BundleApplyMappingsValue, BundleManifestValue, ResolvedInstallationValue,
};
use crate::core::bundle::{
    BundleAddonLockApplyRequest as DomainBundleAddonLockApplyRequest,
    BundleApplyMappings as DomainBundleApplyMappings, PackBundleRequest as DomainPackBundleRequest,
    UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone)]
pub struct InspectBundleRequest {
    pub bundle_path: PathBuf,
}

impl InspectBundleRequest {
    pub(crate) fn into_bundle_path(self, runtime: &AppRuntime) -> AppResult<PathBuf> {
        resolve_bundle_path(runtime, self.bundle_path)
    }
}

#[derive(Debug, Clone)]
pub struct PackBundleAppRequest {
    pub installation: ResolvedInstallationValue,
    pub manifest: BundleManifestValue,
    pub output_path: Option<PathBuf>,
    pub manifest_base_dir: Option<PathBuf>,
}

impl RuntimeDefaultableRequest for PackBundleAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_bundle_output_default(runtime, &mut self.output_path);
        self
    }
}

impl PackBundleAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainPackBundleRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            Ok(DomainPackBundleRequest {
                installation: request.installation.into_domain(),
                addon_state_storage_kind: runtime.addon_state_storage_kind(),
                manifest: request.manifest.into_domain(),
                output_path: request.output_path,
                manifest_base_dir: resolve_optional_app_input_path(
                    runtime,
                    request.manifest_base_dir,
                    "bundle manifest base directory",
                )?,
            })
        })
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
        runtime: &AppRuntime,
    ) -> AppResult<(
        PathBuf,
        DetectedFlavorInstallation,
        DomainBundleApplyMappings,
    )> {
        Ok((
            resolve_bundle_path(runtime, self.bundle_path)?,
            self.installation.into_domain(),
            self.apply_mappings.into_domain(),
        ))
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

impl RuntimeDefaultableRequest for ApplyBundleAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl ApplyBundleAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainUnpackBundleRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            Ok(DomainUnpackBundleRequest {
                bundle_path: resolve_bundle_path(runtime, request.bundle_path)?,
                installation: request.installation.into_domain(),
                dry_run: request.dry_run,
                backup_output_path: resolve_bundle_backup_output_path(
                    runtime,
                    request.backup_output_path,
                )?,
                apply_mappings: request.apply_mappings.into_domain(),
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct PlanBundleAddonLockRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
}

impl PlanBundleAddonLockRequest {
    pub(crate) fn into_domain_inputs(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<(PathBuf, DetectedFlavorInstallation)> {
        Ok((
            resolve_bundle_path(runtime, self.bundle_path)?,
            self.installation.into_domain(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct ApplyBundleAddonLockAppRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl RuntimeDefaultableRequest for ApplyBundleAddonLockAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl ApplyBundleAddonLockAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainBundleAddonLockApplyRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            Ok(DomainBundleAddonLockApplyRequest {
                bundle_path: resolve_bundle_path(runtime, request.bundle_path)?,
                installation: request.installation.into_domain(),
                addon_state_storage_kind: runtime.addon_state_storage_kind(),
                backup_output_path: resolve_bundle_backup_output_path(
                    runtime,
                    request.backup_output_path,
                )?,
                replace_existing: request.replace_existing,
            })
        })
    }
}

fn resolve_bundle_path(runtime: &AppRuntime, path: PathBuf) -> AppResult<PathBuf> {
    resolve_app_input_path(runtime, path, "bundle archive")
}

fn resolve_bundle_backup_output_path(
    runtime: &AppRuntime,
    path: Option<PathBuf>,
) -> AppResult<Option<PathBuf>> {
    resolve_optional_app_output_path(runtime, path, "bundle backup output directory")
}
