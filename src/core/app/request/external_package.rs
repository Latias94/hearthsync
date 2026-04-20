use std::path::PathBuf;

use super::{
    RuntimeDefaultableRequest, apply_backup_output_default, apply_bundle_output_default,
    apply_source_platform_default,
};
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

impl AnalyzeExternalPackageAppRequest {
    pub(crate) fn into_domain_request(self) -> DomainAnalyzeExternalPackageRequest {
        DomainAnalyzeExternalPackageRequest {
            source_path: self.source_path,
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

impl RuntimeDefaultableRequest for CreateExternalPackageBundleAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_source_platform_default(runtime, &mut self.source_platform);
        apply_bundle_output_default(runtime, &mut self.output_path);
        self
    }
}

impl CreateExternalPackageBundleAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainCreateExternalPackageBundleRequest {
        self.into_domain_with_runtime_defaults(runtime, Self::into_domain_request_after_defaults)
    }

    fn into_domain_request_after_defaults(self) -> DomainCreateExternalPackageBundleRequest {
        DomainCreateExternalPackageBundleRequest {
            source_path: self.source_path,
            source_flavor: self.source_flavor.into_domain(),
            source_platform: self.source_platform.map(HostPlatformValue::into_domain),
            supported_targets: self
                .supported_targets
                .into_iter()
                .map(WowFlavorValue::into_domain)
                .collect(),
            output_path: self.output_path,
            package_id: self.package_id,
            package_name: self.package_name,
            created_by: self.created_by,
            description: self.description,
            apply_defaults: self
                .apply_defaults
                .map(BundleApplyDefaultsValue::into_domain),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanExternalPackageApplyAppRequest {
    pub external_package: CreateExternalPackageBundleAppRequest,
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl RuntimeDefaultableRequest for PlanExternalPackageApplyAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.external_package = self.external_package.apply_runtime_defaults(runtime);
        self
    }
}

impl PlanExternalPackageApplyAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainPlanExternalPackageApplyRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            DomainPlanExternalPackageApplyRequest {
                external_package: request
                    .external_package
                    .into_domain_request_after_defaults(),
                installation: request.installation.into_domain(),
                apply_mappings: request.apply_mappings.into_domain(),
            }
        })
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

impl RuntimeDefaultableRequest for ApplyExternalPackageAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.external_package = self.external_package.apply_runtime_defaults(runtime);
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl ApplyExternalPackageAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainApplyExternalPackageRequest {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            DomainApplyExternalPackageRequest {
                external_package: request
                    .external_package
                    .into_domain_request_after_defaults(),
                installation: request.installation.into_domain(),
                dry_run: request.dry_run,
                backup_output_path: request.backup_output_path,
                apply_mappings: request.apply_mappings.into_domain(),
            }
        })
    }
}
