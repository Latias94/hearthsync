use std::path::PathBuf;

use super::RuntimeDefaultableRequest;
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

impl From<AnalyzeExternalPackageAppRequest> for InspectConfigAppRequest {
    fn from(value: AnalyzeExternalPackageAppRequest) -> Self {
        Self {
            source_path: value.source_path,
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
    fn apply_runtime_defaults(self, runtime: &AppRuntime) -> Self {
        Self::from(self.into_external_request().apply_runtime_defaults(runtime))
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

impl From<CreateExternalPackageBundleAppRequest> for ConfigPackageAppRequest {
    fn from(value: CreateExternalPackageBundleAppRequest) -> Self {
        Self {
            source_path: value.source_path,
            source_flavor: value.source_flavor,
            source_platform: value.source_platform,
            supported_targets: value.supported_targets,
            output_path: value.output_path,
            package_id: value.package_id,
            package_name: value.package_name,
            created_by: value.created_by,
            description: value.description,
            apply_defaults: value.apply_defaults,
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
    fn apply_runtime_defaults(self, runtime: &AppRuntime) -> Self {
        Self::from(self.into_external_request().apply_runtime_defaults(runtime))
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

impl From<PlanExternalPackageApplyAppRequest> for PlanConfigApplyAppRequest {
    fn from(value: PlanExternalPackageApplyAppRequest) -> Self {
        Self {
            config_package: ConfigPackageAppRequest::from(value.external_package),
            installation: value.installation,
            apply_mappings: value.apply_mappings,
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
    fn apply_runtime_defaults(self, runtime: &AppRuntime) -> Self {
        Self::from(self.into_external_request().apply_runtime_defaults(runtime))
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

impl From<ApplyExternalPackageAppRequest> for ApplyConfigAppRequest {
    fn from(value: ApplyExternalPackageAppRequest) -> Self {
        Self {
            config_package: ConfigPackageAppRequest::from(value.external_package),
            installation: value.installation,
            dry_run: value.dry_run,
            backup_output_path: value.backup_output_path,
            apply_mappings: value.apply_mappings,
        }
    }
}
