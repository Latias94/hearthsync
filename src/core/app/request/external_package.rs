use std::path::PathBuf;

use crate::core::app::{
    AppRuntime, BundleApplyDefaultsValue, BundleApplyMappingsValue, HostPlatformValue,
    ResolvedInstallationValue, WowFlavorValue,
};
use crate::core::bundle::{
    AnalyzeExternalPackageRequest as DomainAnalyzeExternalPackageRequest,
    ApplyExternalPackageRequest as DomainApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PlanExternalPackageApplyRequest as DomainPlanExternalPackageApplyRequest,
};

#[derive(Debug, Clone)]
pub struct AnalyzeExternalPackageAppRequest {
    pub source_path: PathBuf,
}

impl From<AnalyzeExternalPackageAppRequest> for DomainAnalyzeExternalPackageRequest {
    fn from(request: AnalyzeExternalPackageAppRequest) -> Self {
        Self {
            source_path: request.source_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateExternalPackageBundleAppRequest {
    pub source_path: PathBuf,
    pub source_flavor: WowFlavorValue,
    pub source_platform: Option<HostPlatformValue>,
    pub supported_targets: Vec<WowFlavorValue>,
    pub output_path: Option<PathBuf>,
    pub package_id: Option<String>,
    pub package_name: Option<String>,
    pub created_by: Option<String>,
    pub description: Option<String>,
    pub apply_defaults: Option<BundleApplyDefaultsValue>,
}

impl CreateExternalPackageBundleAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.source_platform = Some(runtime.source_platform_or_host(self.source_platform));
        self.output_path = runtime.bundle_output_or_default(self.output_path);
        self
    }

    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainCreateExternalPackageBundleRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<CreateExternalPackageBundleAppRequest> for DomainCreateExternalPackageBundleRequest {
    fn from(request: CreateExternalPackageBundleAppRequest) -> Self {
        Self {
            source_path: request.source_path,
            source_flavor: request.source_flavor.into(),
            source_platform: request.source_platform.map(Into::into),
            supported_targets: request
                .supported_targets
                .into_iter()
                .map(Into::into)
                .collect(),
            output_path: request.output_path,
            package_id: request.package_id,
            package_name: request.package_name,
            created_by: request.created_by,
            description: request.description,
            apply_defaults: request.apply_defaults.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanExternalPackageApplyAppRequest {
    pub external_package: CreateExternalPackageBundleAppRequest,
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl PlanExternalPackageApplyAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.external_package = self.external_package.apply_runtime_defaults(runtime);
        self
    }

    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainPlanExternalPackageApplyRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<PlanExternalPackageApplyAppRequest> for DomainPlanExternalPackageApplyRequest {
    fn from(request: PlanExternalPackageApplyAppRequest) -> Self {
        Self {
            external_package: request.external_package.into(),
            installation: request.installation.into(),
            apply_mappings: request.apply_mappings.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplyExternalPackageAppRequest {
    pub external_package: CreateExternalPackageBundleAppRequest,
    pub installation: ResolvedInstallationValue,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl ApplyExternalPackageAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.external_package = self.external_package.apply_runtime_defaults(runtime);
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainApplyExternalPackageRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<ApplyExternalPackageAppRequest> for DomainApplyExternalPackageRequest {
    fn from(request: ApplyExternalPackageAppRequest) -> Self {
        Self {
            external_package: request.external_package.into(),
            installation: request.installation.into(),
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            apply_mappings: request.apply_mappings.into(),
        }
    }
}
