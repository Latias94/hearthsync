use std::path::PathBuf;

use super::{
    RuntimeDefaultableRequest, apply_backup_output_default, apply_bundle_output_default,
    apply_source_platform_default,
};
use crate::core::app::request::external_package::{
    AnalyzeExternalPackageAppRequest, ApplyExternalPackageAppRequest,
    CreateExternalPackageBundleAppRequest, PlanExternalPackageApplyAppRequest,
};
use crate::core::app::{
    AppRuntime, BundleApplyDefaultsValue, BundleApplyMappingsValue, HostPlatformValue,
    ResolvedInstallationValue, WowFlavorValue,
};

#[derive(Debug, Clone)]
pub struct InspectConfigAppRequest {
    pub source_path: PathBuf,
}

impl InspectConfigAppRequest {
    pub(crate) fn into_external_request(self) -> AnalyzeExternalPackageAppRequest {
        AnalyzeExternalPackageAppRequest {
            source_path: self.source_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigPackageAppRequest {
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

impl RuntimeDefaultableRequest for ConfigPackageAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        apply_source_platform_default(runtime, &mut self.source_platform);
        apply_bundle_output_default(runtime, &mut self.output_path);
        self
    }
}

impl ConfigPackageAppRequest {
    pub(crate) fn into_external_request(self) -> CreateExternalPackageBundleAppRequest {
        CreateExternalPackageBundleAppRequest {
            source_path: self.source_path,
            source_flavor: self.source_flavor,
            source_platform: self.source_platform,
            supported_targets: self.supported_targets,
            output_path: self.output_path,
            package_id: self.package_id,
            package_name: self.package_name,
            created_by: self.created_by,
            description: self.description,
            apply_defaults: self.apply_defaults,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanConfigApplyAppRequest {
    pub config_package: ConfigPackageAppRequest,
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl RuntimeDefaultableRequest for PlanConfigApplyAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.config_package = self.config_package.apply_runtime_defaults(runtime);
        self
    }
}

impl PlanConfigApplyAppRequest {
    pub(crate) fn into_external_request(self) -> PlanExternalPackageApplyAppRequest {
        PlanExternalPackageApplyAppRequest {
            external_package: self.config_package.into_external_request(),
            installation: self.installation,
            apply_mappings: self.apply_mappings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplyConfigAppRequest {
    pub config_package: ConfigPackageAppRequest,
    pub installation: ResolvedInstallationValue,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl RuntimeDefaultableRequest for ApplyConfigAppRequest {
    fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.config_package = self.config_package.apply_runtime_defaults(runtime);
        apply_backup_output_default(runtime, &mut self.backup_output_path);
        self
    }
}

impl ApplyConfigAppRequest {
    pub(crate) fn into_external_request(self) -> ApplyExternalPackageAppRequest {
        ApplyExternalPackageAppRequest {
            external_package: self.config_package.into_external_request(),
            installation: self.installation,
            dry_run: self.dry_run,
            backup_output_path: self.backup_output_path,
            apply_mappings: self.apply_mappings,
        }
    }
}
