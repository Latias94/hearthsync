use std::path::PathBuf;

use super::{
    AddonPackageMetadataValue, AppRuntime, BackupGroupValue, BundleApplyDefaultsValue,
    BundleApplyMappingsValue, BundleManifestValue, HostPlatformValue, ResolvedInstallationValue,
    WowFlavorValue,
};
use crate::core::addon::index::{
    AddonIndexInstallRequest as DomainAddonIndexInstallRequest,
    AddonIndexUpdateRequest as DomainAddonIndexUpdateRequest,
};
use crate::core::addon::lock::{
    AddonLockApplyRequest as DomainAddonLockApplyRequest,
    AddonLockSourceOverride as DomainAddonLockSourceOverride,
};
use crate::core::addon::{
    InstallAddonRequest as DomainInstallAddonRequest,
    RemoveAddonRequest as DomainRemoveAddonRequest, SearchAddonRequest as DomainSearchAddonRequest,
    UpdateAddonRequest as DomainUpdateAddonRequest,
};
use crate::core::backup::{
    BackupRequest as DomainBackupRequest, RestoreBackupRequest as DomainRestoreBackupRequest,
};
use crate::core::bundle::{
    AnalyzeExternalPackageRequest as DomainAnalyzeExternalPackageRequest,
    ApplyExternalPackageRequest as DomainApplyExternalPackageRequest,
    BundleAddonLockApplyRequest as DomainBundleAddonLockApplyRequest,
    BundleApplyMappings as DomainBundleApplyMappings,
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PackBundleRequest as DomainPackBundleRequest,
    PlanExternalPackageApplyRequest as DomainPlanExternalPackageApplyRequest,
    UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::error::AppResult;
use crate::core::install::{
    DetectedFlavorInstallation, ProductInstallInspection, inspect_installation_on_host,
    resolve_installation_on_host,
};

#[derive(Debug, Clone)]
pub struct SearchAddonsRequest {
    pub installation: ResolvedInstallationValue,
    pub query: String,
    pub limit: usize,
}

impl From<SearchAddonsRequest> for DomainSearchAddonRequest {
    fn from(request: SearchAddonsRequest) -> Self {
        Self {
            installation: request.installation.into(),
            query: request.query,
            limit: request.limit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListAddonsRequest {
    pub installation: ResolvedInstallationValue,
}

impl ListAddonsRequest {
    pub(crate) fn into_domain_installation(self) -> DetectedFlavorInstallation {
        self.installation.into()
    }
}

#[derive(Debug, Clone)]
pub struct InspectAddonIndexRequest {
    pub index_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InspectAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

impl InspectAddonLockRequest {
    pub(crate) fn into_domain_installation(self) -> DetectedFlavorInstallation {
        self.installation.into()
    }
}

#[derive(Debug, Clone)]
pub struct WriteAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

impl WriteAddonLockRequest {
    pub(crate) fn into_domain_installation(self) -> DetectedFlavorInstallation {
        self.installation.into()
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
        (self.installation.into(), self.lock_path)
    }
}

#[derive(Debug, Clone)]
pub struct PlanAddonLockSyncRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
}

impl PlanAddonLockSyncRequest {
    pub(crate) fn into_domain_inputs(self) -> (DetectedFlavorInstallation, Option<PathBuf>) {
        (self.installation.into(), self.lock_path)
    }
}

#[derive(Debug, Clone)]
pub struct AddonLockSourceOverrideRequest {
    pub comparison_key: String,
    pub archive_path: PathBuf,
}

impl From<AddonLockSourceOverrideRequest> for DomainAddonLockSourceOverride {
    fn from(request: AddonLockSourceOverrideRequest) -> Self {
        Self {
            comparison_key: request.comparison_key,
            archive_path: request.archive_path,
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
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<ApplyAddonLockAppRequest> for DomainAddonLockApplyRequest {
    fn from(request: ApplyAddonLockAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            lock_path: request.lock_path,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            source_overrides: request
                .source_overrides
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub source: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub metadata: Option<AddonPackageMetadataValue>,
}

impl InstallAddonAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainInstallAddonRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<InstallAddonAppRequest> for DomainInstallAddonRequest {
    fn from(request: InstallAddonAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            source: request.source,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            metadata: request.metadata.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl UpdateAddonAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainUpdateAddonRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<UpdateAddonAppRequest> for DomainUpdateAddonRequest {
    fn from(request: UpdateAddonAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoveAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl RemoveAddonAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainRemoveAddonRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<RemoveAddonAppRequest> for DomainRemoveAddonRequest {
    fn from(request: RemoveAddonAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl InstallAddonIndexAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> DomainAddonIndexInstallRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<InstallAddonIndexAppRequest> for DomainAddonIndexInstallRequest {
    fn from(request: InstallAddonIndexAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl UpdateAddonIndexAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_output_path = runtime.backup_output_or_default(self.backup_output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainAddonIndexUpdateRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<UpdateAddonIndexAppRequest> for DomainAddonIndexUpdateRequest {
    fn from(request: UpdateAddonIndexAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListBackupsRequest {
    pub backup_dir: Option<PathBuf>,
}

impl ListBackupsRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_dir = runtime.backup_dir_or_default(self.backup_dir);
        self
    }

    pub(crate) fn into_backup_dir(self, runtime: &AppRuntime) -> Option<PathBuf> {
        self.apply_runtime_defaults(runtime).backup_dir
    }
}

#[derive(Debug, Clone)]
pub struct CreateBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub output_path: Option<PathBuf>,
    pub groups: Vec<BackupGroupValue>,
    pub label: Option<String>,
}

impl CreateBackupAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.output_path = runtime.backup_output_or_default(self.output_path);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainBackupRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<CreateBackupAppRequest> for DomainBackupRequest {
    fn from(request: CreateBackupAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            output_path: request.output_path,
            groups: request.groups.into_iter().map(Into::into).collect(),
            label: request.label,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RestoreBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub archive_path: Option<PathBuf>,
    pub backup_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
}

impl RestoreBackupAppRequest {
    pub(crate) fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
        self.backup_dir = runtime.backup_dir_or_default(self.backup_dir);
        self
    }

    pub(crate) fn into_domain_request(self, runtime: &AppRuntime) -> DomainRestoreBackupRequest {
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<RestoreBackupAppRequest> for DomainRestoreBackupRequest {
    fn from(request: RestoreBackupAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            archive_path: request.archive_path,
            backup_id: request.backup_id,
            backup_dir: request.backup_dir,
        }
    }
}

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
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<PackBundleAppRequest> for DomainPackBundleRequest {
    fn from(request: PackBundleAppRequest) -> Self {
        Self {
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
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<ApplyBundleAppRequest> for DomainUnpackBundleRequest {
    fn from(request: ApplyBundleAppRequest) -> Self {
        Self {
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
        self.apply_runtime_defaults(runtime).into()
    }
}

impl From<ApplyBundleAddonLockAppRequest> for DomainBundleAddonLockApplyRequest {
    fn from(request: ApplyBundleAddonLockAppRequest) -> Self {
        Self {
            bundle_path: request.bundle_path,
            installation: request.installation.into(),
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
    }
}

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
    pub fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
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
    pub fn apply_runtime_defaults(mut self, runtime: &AppRuntime) -> Self {
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

#[derive(Debug, Clone)]
pub struct InspectInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavorValue>,
}

impl InspectInstallationRequest {
    pub(crate) fn inspect_with_runtime(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<ProductInstallInspection> {
        inspect_installation_on_host(
            &self.path,
            self.flavor.map(Into::into),
            runtime.host_platform().into(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct ResolveInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavorValue>,
}

impl ResolveInstallationRequest {
    pub(crate) fn resolve_with_runtime(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DetectedFlavorInstallation> {
        resolve_installation_on_host(
            &self.path,
            self.flavor.map(Into::into),
            runtime.host_platform().into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::app::{
        AddonPackageMetadataValue, AppRuntime, BackupGroupValue, BundleApplyDefaultsValue,
        BundleApplyMappingsValue, BundleCharacterMappingOverrideValue,
        BundleCharacterResourceValue, BundleManifestValue, BundleMappingRulesValue,
        BundlePackageValue, BundleResourcesValue, BundleSourceValue, CharacterMappingModeValue,
        HostPlatformValue, ResourceApplyPolicyValue, WowFlavorValue,
    };
    use crate::core::manifest::{CharacterMappingMode, ResourceApplyPolicy};

    #[test]
    fn addon_family_requests_apply_runtime_backup_defaults() {
        let runtime =
            AppRuntime::new().with_default_backup_dir(Some(PathBuf::from("runtime-backups")));

        let install = InstallAddonAppRequest {
            installation: sample_installation(),
            source: "https://example.invalid/weakauras.zip".to_string(),
            dry_run: false,
            backup_output_path: None,
            replace_existing: true,
            metadata: None,
        }
        .apply_runtime_defaults(&runtime);
        let update = UpdateAddonAppRequest {
            installation: sample_installation(),
            name: Some("WeakAuras".to_string()),
            dry_run: false,
            backup_output_path: None,
        }
        .apply_runtime_defaults(&runtime);
        let remove = RemoveAddonAppRequest {
            installation: sample_installation(),
            name: "WeakAuras".to_string(),
            dry_run: false,
            backup_output_path: None,
        }
        .apply_runtime_defaults(&runtime);
        let index_install = InstallAddonIndexAppRequest {
            installation: sample_installation(),
            index_path: PathBuf::from("addon-index.toml"),
            name: "WeakAuras".to_string(),
            dry_run: false,
            backup_output_path: None,
            replace_existing: true,
        }
        .apply_runtime_defaults(&runtime);
        let index_update = UpdateAddonIndexAppRequest {
            installation: sample_installation(),
            index_path: PathBuf::from("addon-index.toml"),
            name: None,
            dry_run: false,
            backup_output_path: None,
        }
        .apply_runtime_defaults(&runtime);
        let lock_apply = ApplyAddonLockAppRequest {
            installation: sample_installation(),
            lock_path: None,
            backup_output_path: None,
            replace_existing: true,
            source_overrides: Vec::new(),
        }
        .apply_runtime_defaults(&runtime);

        assert_eq!(
            install.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
        assert_eq!(
            update.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
        assert_eq!(
            remove.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
        assert_eq!(
            index_install.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
        assert_eq!(
            index_update.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
        assert_eq!(
            lock_apply.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
    }

    #[test]
    fn backup_requests_apply_runtime_defaults() {
        let runtime =
            AppRuntime::new().with_default_backup_dir(Some(PathBuf::from("runtime-backups")));

        let list = ListBackupsRequest { backup_dir: None }.apply_runtime_defaults(&runtime);
        let create = CreateBackupAppRequest {
            installation: sample_installation(),
            output_path: None,
            groups: vec![BackupGroupValue::Addons],
            label: Some("nightly".to_string()),
        }
        .apply_runtime_defaults(&runtime);
        let restore = RestoreBackupAppRequest {
            installation: sample_installation(),
            archive_path: None,
            backup_id: Some("backup-001".to_string()),
            backup_dir: None,
        }
        .apply_runtime_defaults(&runtime);

        assert_eq!(list.backup_dir, Some(PathBuf::from("runtime-backups")));
        assert_eq!(create.output_path, Some(PathBuf::from("runtime-backups")));
        assert_eq!(restore.backup_dir, Some(PathBuf::from("runtime-backups")));
    }

    #[test]
    fn bundle_requests_apply_runtime_defaults() {
        let runtime = AppRuntime::new()
            .with_default_backup_dir(Some(PathBuf::from("runtime-backups")))
            .with_default_bundle_output_dir(Some(PathBuf::from("runtime-bundles")));

        let pack = PackBundleAppRequest {
            installation: sample_installation(),
            manifest: sample_manifest(),
            output_path: None,
            manifest_base_dir: None,
        }
        .apply_runtime_defaults(&runtime);
        let apply = ApplyBundleAppRequest {
            bundle_path: PathBuf::from("bundle.zip"),
            installation: sample_installation(),
            dry_run: false,
            backup_output_path: None,
            apply_mappings: BundleApplyMappingsValue::default(),
        }
        .apply_runtime_defaults(&runtime);
        let addon_lock = ApplyBundleAddonLockAppRequest {
            bundle_path: PathBuf::from("bundle.zip"),
            installation: sample_installation(),
            backup_output_path: None,
            replace_existing: true,
        }
        .apply_runtime_defaults(&runtime);

        assert_eq!(pack.output_path, Some(PathBuf::from("runtime-bundles")));
        assert_eq!(
            apply.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
        assert_eq!(
            addon_lock.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
    }

    #[test]
    fn external_package_requests_apply_runtime_defaults() {
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_default_backup_dir(Some(PathBuf::from("runtime-backups")))
            .with_default_bundle_output_dir(Some(PathBuf::from("runtime-bundles")));

        let bundle_request =
            sample_external_package_bundle_request().apply_runtime_defaults(&runtime);
        let plan_request = PlanExternalPackageApplyAppRequest {
            external_package: sample_external_package_bundle_request(),
            installation: sample_installation(),
            apply_mappings: BundleApplyMappingsValue::default(),
        }
        .apply_runtime_defaults(&runtime);
        let apply_request = ApplyExternalPackageAppRequest {
            external_package: sample_external_package_bundle_request(),
            installation: sample_installation(),
            dry_run: false,
            backup_output_path: None,
            apply_mappings: BundleApplyMappingsValue::default(),
        }
        .apply_runtime_defaults(&runtime);

        assert_eq!(
            bundle_request.source_platform,
            Some(HostPlatformValue::MacOs)
        );
        assert_eq!(
            bundle_request.output_path,
            Some(PathBuf::from("runtime-bundles"))
        );
        assert_eq!(
            plan_request.external_package.source_platform,
            Some(HostPlatformValue::MacOs)
        );
        assert_eq!(
            apply_request.external_package.output_path,
            Some(PathBuf::from("runtime-bundles"))
        );
        assert_eq!(
            apply_request.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
    }

    #[test]
    fn runtime_backed_request_helpers_compose_defaults_and_domain_projection() {
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_default_backup_dir(Some(PathBuf::from("runtime-backups")))
            .with_default_bundle_output_dir(Some(PathBuf::from("runtime-bundles")));

        let install = InstallAddonAppRequest {
            installation: sample_installation(),
            source: "https://example.invalid/weakauras.zip".to_string(),
            dry_run: false,
            backup_output_path: None,
            replace_existing: true,
            metadata: None,
        }
        .into_domain_request(&runtime);
        let backup_dir = ListBackupsRequest { backup_dir: None }.into_backup_dir(&runtime);
        let external_bundle =
            sample_external_package_bundle_request().into_domain_request(&runtime);

        assert_eq!(
            install.backup_output_path,
            Some(PathBuf::from("runtime-backups"))
        );
        assert_eq!(backup_dir, Some(PathBuf::from("runtime-backups")));
        assert_eq!(
            external_bundle.source_platform,
            Some(crate::core::install::HostPlatform::MacOs)
        );
        assert_eq!(
            external_bundle.output_path,
            Some(PathBuf::from("runtime-bundles"))
        );
    }

    #[test]
    fn thin_installation_requests_project_domain_inputs() {
        let installation = sample_installation();
        let domain_installation = ListAddonsRequest {
            installation: installation.clone(),
        }
        .into_domain_installation();
        let (lock_installation, lock_path) = PlanAddonLockSyncRequest {
            installation: installation.clone(),
            lock_path: Some(PathBuf::from("lock.toml")),
        }
        .into_domain_inputs();
        let (bundle_path, bundle_installation, apply_mappings) = PlanBundleApplyRequest {
            bundle_path: PathBuf::from("bundle.zip"),
            installation,
            apply_mappings: BundleApplyMappingsValue {
                target_account: Some("AccountA".to_string()),
                target_server: Some("Illidan".to_string()),
                target_character: Some("Main".to_string()),
                selected_accounts: vec!["AccountA".to_string()],
                all_accounts: false,
                characters: Vec::new(),
            },
        }
        .into_domain_inputs();

        assert_eq!(
            domain_installation.product_root,
            PathBuf::from("World of Warcraft")
        );
        assert_eq!(
            lock_installation.flavor_root,
            PathBuf::from("World of Warcraft/_retail_")
        );
        assert_eq!(lock_path, Some(PathBuf::from("lock.toml")));
        assert_eq!(bundle_path, PathBuf::from("bundle.zip"));
        assert_eq!(
            bundle_installation.addon_dir,
            PathBuf::from("World of Warcraft/_retail_/Interface/AddOns")
        );
        assert_eq!(apply_mappings.target_account.as_deref(), Some("AccountA"));
        assert_eq!(apply_mappings.target_server.as_deref(), Some("Illidan"));
        assert_eq!(apply_mappings.target_character.as_deref(), Some("Main"));
    }

    #[test]
    fn apply_bundle_request_converts_app_owned_apply_mappings() {
        let domain: DomainUnpackBundleRequest = ApplyBundleAppRequest {
            bundle_path: PathBuf::from("bundle.zip"),
            installation: sample_installation(),
            dry_run: true,
            backup_output_path: Some(PathBuf::from("backup")),
            apply_mappings: BundleApplyMappingsValue {
                target_account: Some("AccountA".to_string()),
                target_server: Some("Illidan".to_string()),
                target_character: Some("Main".to_string()),
                selected_accounts: vec!["AccountA".to_string()],
                all_accounts: true,
                characters: vec![BundleCharacterMappingOverrideValue {
                    source_account: Some("SourceAccount".to_string()),
                    source_server: "Stormrage".to_string(),
                    source_character: "SourceMain".to_string(),
                    target_account: Some("TargetAccount".to_string()),
                    target_server: "Illidan".to_string(),
                    target_character: "TargetMain".to_string(),
                }],
            },
        }
        .into();

        assert_eq!(domain.bundle_path, PathBuf::from("bundle.zip"));
        assert!(domain.dry_run);
        assert_eq!(
            domain.apply_mappings.target_account.as_deref(),
            Some("AccountA")
        );
        assert_eq!(
            domain.apply_mappings.target_server.as_deref(),
            Some("Illidan")
        );
        assert_eq!(
            domain.apply_mappings.target_character.as_deref(),
            Some("Main")
        );
        assert_eq!(domain.apply_mappings.selected_accounts, vec!["AccountA"]);
        assert!(domain.apply_mappings.all_accounts);
        assert_eq!(domain.apply_mappings.characters.len(), 1);
        assert_eq!(
            domain.apply_mappings.characters[0]
                .source_account
                .as_deref(),
            Some("SourceAccount")
        );
    }

    #[test]
    fn create_external_package_request_converts_app_owned_apply_defaults() {
        let domain: DomainCreateExternalPackageBundleRequest =
            CreateExternalPackageBundleAppRequest {
                source_path: PathBuf::from("author-ui.zip"),
                source_flavor: WowFlavorValue::Retail,
                source_platform: Some(HostPlatformValue::Windows),
                supported_targets: vec![WowFlavorValue::Retail, WowFlavorValue::Classic],
                output_path: Some(PathBuf::from("out")),
                package_id: Some("author-ui".to_string()),
                package_name: Some("Author UI".to_string()),
                created_by: Some("tester".to_string()),
                description: Some("normalized".to_string()),
                apply_defaults: Some(BundleApplyDefaultsValue {
                    create_backup: false,
                    addons: ResourceApplyPolicyValue::Mirror,
                    wtf_common: ResourceApplyPolicyValue::Share,
                    wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
                    fonts: ResourceApplyPolicyValue::Preserve,
                    interface_assets: ResourceApplyPolicyValue::Sync,
                }),
            }
            .into();

        let apply_defaults = domain.apply_defaults.expect("apply defaults");
        assert!(!apply_defaults.create_backup);
        assert_eq!(apply_defaults.addons, ResourceApplyPolicy::Mirror);
        assert_eq!(apply_defaults.wtf_common, ResourceApplyPolicy::Share);
        assert_eq!(
            apply_defaults.wtf_characters,
            ResourceApplyPolicy::ReplaceSelected
        );
        assert_eq!(apply_defaults.fonts, ResourceApplyPolicy::Preserve);
        assert_eq!(apply_defaults.interface_assets, ResourceApplyPolicy::Sync);
    }

    #[test]
    fn pack_bundle_request_converts_app_owned_manifest() {
        let domain: DomainPackBundleRequest = PackBundleAppRequest {
            installation: sample_installation(),
            manifest: sample_manifest(),
            output_path: Some(PathBuf::from("bundle.zip")),
            manifest_base_dir: Some(PathBuf::from("manifest-dir")),
        }
        .into();

        assert_eq!(domain.manifest.schema_version, 1);
        assert_eq!(domain.manifest.package.id, "author-ui");
        assert_eq!(
            domain.manifest.source.flavor,
            crate::core::install::WowFlavor::Retail
        );
        assert_eq!(domain.manifest.resources.addons, vec!["WeakAuras"]);
        assert_eq!(domain.manifest.resources.wtf_characters.len(), 1);
        assert_eq!(
            domain.manifest.mapping.character_mode,
            CharacterMappingMode::Explicit
        );
        assert_eq!(domain.manifest.apply.addons, ResourceApplyPolicy::Mirror);
    }

    #[test]
    fn install_addon_request_converts_app_owned_metadata() {
        let domain: DomainInstallAddonRequest = InstallAddonAppRequest {
            installation: sample_installation(),
            source: "https://example.invalid/weakauras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(PathBuf::from("backup")),
            replace_existing: true,
            metadata: Some(AddonPackageMetadataValue {
                index_name: Some("curated".to_string()),
                index_package_id: Some("weakauras".to_string()),
                package_name: Some("WeakAuras".to_string()),
                version: Some("1.2.3".to_string()),
                source_url: Some("https://example.invalid/weakauras.zip".to_string()),
                website_url: Some("https://example.invalid/weakauras".to_string()),
                source_sha256: Some("abc123".to_string()),
                supported_flavors: vec!["retail".to_string()],
            }),
        }
        .into();

        let metadata = domain.metadata.expect("metadata");
        assert_eq!(metadata.index_name.as_deref(), Some("curated"));
        assert_eq!(metadata.index_package_id.as_deref(), Some("weakauras"));
        assert_eq!(metadata.package_name.as_deref(), Some("WeakAuras"));
        assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
        assert_eq!(
            metadata.source_url.as_deref(),
            Some("https://example.invalid/weakauras.zip")
        );
        assert_eq!(metadata.supported_flavors, vec!["retail"]);
    }

    fn sample_installation() -> ResolvedInstallationValue {
        ResolvedInstallationValue {
            platform: HostPlatformValue::Windows,
            flavor: WowFlavorValue::Retail,
            product_root: PathBuf::from("World of Warcraft"),
            flavor_root: PathBuf::from("World of Warcraft/_retail_"),
            interface_dir: PathBuf::from("World of Warcraft/_retail_/Interface"),
            addon_dir: PathBuf::from("World of Warcraft/_retail_/Interface/AddOns"),
            wtf_dir: PathBuf::from("World of Warcraft/_retail_/WTF"),
            fonts_dir: PathBuf::from("World of Warcraft/_retail_/Fonts"),
        }
    }

    fn sample_manifest() -> BundleManifestValue {
        BundleManifestValue {
            schema_version: 1,
            package: BundlePackageValue {
                id: "author-ui".to_string(),
                name: "Author UI".to_string(),
                created_by: "tester".to_string(),
                description: Some("fixture".to_string()),
            },
            source: BundleSourceValue {
                flavor: WowFlavorValue::Retail,
                platform: Some(HostPlatformValue::Windows),
                exported_at: None,
                supported_targets: vec![WowFlavorValue::Retail],
            },
            resources: BundleResourcesValue {
                addons: vec!["WeakAuras".to_string()],
                wtf_common: true,
                wtf_characters: vec![BundleCharacterResourceValue {
                    source_account: Some("AccountA".to_string()),
                    source_server: "Illidan".to_string(),
                    source_character: "Main".to_string(),
                    target_hint: Some("Main".to_string()),
                }],
                fonts: true,
                interface_assets: vec!["Interface/Buttons".to_string()],
                addon_lock: false,
                addon_indexes: Vec::new(),
            },
            mapping: BundleMappingRulesValue {
                character_mode: CharacterMappingModeValue::Explicit,
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
                allow_cross_platform: true,
            },
            apply: BundleApplyDefaultsValue {
                create_backup: true,
                addons: ResourceApplyPolicyValue::Mirror,
                wtf_common: ResourceApplyPolicyValue::Share,
                wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
                fonts: ResourceApplyPolicyValue::Mirror,
                interface_assets: ResourceApplyPolicyValue::Mirror,
            },
        }
    }

    fn sample_external_package_bundle_request() -> CreateExternalPackageBundleAppRequest {
        CreateExternalPackageBundleAppRequest {
            source_path: PathBuf::from("author-ui.zip"),
            source_flavor: WowFlavorValue::Retail,
            source_platform: None,
            supported_targets: vec![WowFlavorValue::Retail],
            output_path: None,
            package_id: Some("author-ui".to_string()),
            package_name: Some("Author UI".to_string()),
            created_by: Some("tester".to_string()),
            description: Some("fixture".to_string()),
            apply_defaults: None,
        }
    }
}
