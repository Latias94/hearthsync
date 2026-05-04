use std::path::PathBuf;

use super::super::map_owned_vec;
use super::{
    RuntimeDefaultableRequest, apply_backup_output_default, apply_bundle_output_default,
    apply_source_platform_default, resolve_app_input_path, resolve_optional_app_output_path,
};
use crate::core::app::{
    AppRuntime, BundleApplyDefaultsValue, BundleApplyMappingsValue, ExternalPackageLayoutValue,
    HostPlatformValue, ResolvedInstallationValue, WowFlavorValue,
};
use crate::core::bundle::{
    AnalyzeExternalPackageRequest as DomainAnalyzeExternalPackageRequest,
    ApplyExternalPackageRequest as DomainApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PlanExternalPackageApplyRequest as DomainPlanExternalPackageApplyRequest,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone)]
pub struct AnalyzeExternalPackageAppRequest {
    pub source_path: PathBuf,
    pub layout: ExternalPackageLayoutValue,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

impl AnalyzeExternalPackageAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainAnalyzeExternalPackageRequest> {
        Ok(DomainAnalyzeExternalPackageRequest {
            source_path: resolve_external_package_source_path(runtime, self.source_path)?,
            layout: self.layout.into_domain(),
            source_account: self.source_account,
            source_server: self.source_server,
            source_character: self.source_character,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CreateExternalPackageBundleAppRequest {
    pub source_path: PathBuf,
    pub layout: ExternalPackageLayoutValue,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
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
    ) -> AppResult<DomainCreateExternalPackageBundleRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            request.into_domain_request_after_defaults(runtime)
        })
    }

    fn into_domain_request_after_defaults(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainCreateExternalPackageBundleRequest> {
        Ok(DomainCreateExternalPackageBundleRequest {
            source_path: resolve_external_package_source_path(runtime, self.source_path)?,
            layout: self.layout.into_domain(),
            source_account: self.source_account,
            source_server: self.source_server,
            source_character: self.source_character,
            source_flavor: self.source_flavor.into_domain(),
            source_platform: self.source_platform.map(HostPlatformValue::into_domain),
            supported_targets: map_owned_vec(self.supported_targets, WowFlavorValue::into_domain),
            output_path: resolve_optional_app_output_path(
                runtime,
                self.output_path,
                "external package bundle output",
            )?,
            package_id: self.package_id,
            package_name: self.package_name,
            created_by: self.created_by,
            description: self.description,
            apply_defaults: self
                .apply_defaults
                .map(BundleApplyDefaultsValue::into_domain),
        })
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
    ) -> AppResult<DomainPlanExternalPackageApplyRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            Ok(DomainPlanExternalPackageApplyRequest {
                external_package: request
                    .external_package
                    .into_domain_request_after_defaults(runtime)?,
                installation: request.installation.into_domain()?,
                apply_mappings: request.apply_mappings.into_domain()?,
            })
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
    ) -> AppResult<DomainApplyExternalPackageRequest> {
        self.into_domain_with_runtime_defaults(runtime, |request| {
            Ok(DomainApplyExternalPackageRequest {
                external_package: request
                    .external_package
                    .into_domain_request_after_defaults(runtime)?,
                installation: request.installation.into_domain()?,
                dry_run: request.dry_run,
                backup_output_path: resolve_optional_app_output_path(
                    runtime,
                    request.backup_output_path,
                    "external package backup output directory",
                )?,
                apply_mappings: request.apply_mappings.into_domain()?,
            })
        })
    }
}

fn resolve_external_package_source_path(runtime: &AppRuntime, path: PathBuf) -> AppResult<PathBuf> {
    resolve_app_input_path(runtime, path, "external package source")
}
