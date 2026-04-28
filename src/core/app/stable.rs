use super::{
    AddonPolicyService, AddonService, AppRuntime, AppRuntimeCapabilitiesValue,
    AppRuntimeDiagnosticsValue, BackupService, BundleService, ConfigService,
    ExternalPackageService, InstallationService, ResolvedInstallationValue, RuntimeSettingsService,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone)]
pub struct StableAppServices {
    pub(super) runtime: AppRuntime,
    installations: InstallationService,
    addons: AddonService,
    addon_policies: AddonPolicyService,
    backups: BackupService,
    bundles: BundleService,
    configs: ConfigService,
    external_packages: ExternalPackageService,
    runtime_settings: RuntimeSettingsService,
}

impl Default for StableAppServices {
    fn default() -> Self {
        Self::with_runtime(AppRuntime::default())
    }
}

impl StableAppServices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self {
            installations: InstallationService::with_runtime(runtime.clone()),
            addons: AddonService::with_runtime(runtime.clone()),
            addon_policies: AddonPolicyService::with_runtime(runtime.clone()),
            backups: BackupService::with_runtime(runtime.clone()),
            bundles: BundleService::with_runtime(runtime.clone()),
            external_packages: ExternalPackageService::with_runtime(runtime.clone()),
            runtime_settings: RuntimeSettingsService::with_runtime(runtime.clone()),
            configs: ConfigService::with_external_packages(ExternalPackageService::with_runtime(
                runtime.clone(),
            )),
            runtime,
        }
    }

    #[cfg(test)]
    pub(super) fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn capabilities(&self) -> AppRuntimeCapabilitiesValue {
        self.runtime.capabilities()
    }

    pub fn runtime_diagnostics(&self) -> AppRuntimeDiagnosticsValue {
        self.runtime.diagnostics()
    }

    pub fn runtime_diagnostics_for_installation(
        &self,
        installation: ResolvedInstallationValue,
    ) -> AppResult<AppRuntimeDiagnosticsValue> {
        self.runtime.diagnostics_for_installation(installation)
    }

    pub fn inspect_runtime_settings(&self) -> AppResult<super::RuntimeSettingsInspectionResult> {
        self.runtime_settings().inspect()
    }

    pub fn set_runtime_settings(
        &self,
        request: super::SetRuntimeSettingsAppRequest,
    ) -> AppResult<super::RuntimeSettingsMutationResult> {
        self.runtime_settings().set(request)
    }

    pub fn reset_runtime_settings(&self) -> AppResult<super::RuntimeSettingsMutationResult> {
        self.runtime_settings().reset()
    }

    pub fn scan_installations(&self) -> AppResult<super::InstallationScanResult> {
        self.installations().scan()
    }

    pub fn inspect_installation(
        &self,
        request: super::InspectInstallationRequest,
    ) -> AppResult<super::InstallationInspectionResult> {
        self.installations().inspect(request)
    }

    pub fn resolve_installation(
        &self,
        request: super::ResolveInstallationRequest,
    ) -> AppResult<super::ResolvedInstallationValue> {
        self.installations().resolve(request)
    }

    pub fn search_addons(
        &self,
        request: super::SearchAddonsRequest,
    ) -> AppResult<super::AddonSearchCatalogResult> {
        self.addons().search(request)
    }

    pub fn list_addons(
        &self,
        request: super::ListAddonsRequest,
    ) -> AppResult<super::AddonInventoryResult> {
        self.addons().list(request)
    }

    pub fn adopt_addons(
        &self,
        request: super::AdoptAddonsAppRequest,
    ) -> AppResult<super::AdoptedAddonPackageResult> {
        self.addons().adopt(request)
    }

    pub fn relink_addon(
        &self,
        request: super::RelinkAddonAppRequest,
    ) -> AppResult<super::RelinkedAddonPackageResult> {
        self.addons().relink(request)
    }

    pub fn purge_addon_cache(&self) -> AppResult<super::AddonCachePurgeResult> {
        self.addons().purge_cache()
    }

    pub fn repair_addon_cache(&self) -> AppResult<super::AddonCacheRepairResult> {
        self.addons().repair_cache()
    }

    pub fn install_addon(
        &self,
        request: super::InstallAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::InstalledAddonPackageResult>> {
        self.addons().install_collecting_progress(request)
    }

    pub fn update_addons(
        &self,
        request: super::UpdateAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::UpdatedAddonPackageResult>> {
        self.addons().update_collecting_progress(request)
    }

    pub fn remove_addons(
        &self,
        request: super::RemoveAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::RemovedAddonPackageResult>> {
        self.addons().remove_collecting_progress(request)
    }

    pub fn inspect_addon_policy(
        &self,
        request: super::InspectAddonPolicyRequest,
    ) -> AppResult<super::AddonPolicyInspectionResult> {
        self.addon_policies().inspect(request)
    }

    pub fn set_addon_policy(
        &self,
        request: super::SetAddonPolicyAppRequest,
    ) -> AppResult<super::AddonPolicyMutationResult> {
        self.addon_policies().set(request)
    }

    pub fn remove_addon_policy(
        &self,
        request: super::RemoveAddonPolicyAppRequest,
    ) -> AppResult<super::AddonPolicyMutationResult> {
        self.addon_policies().remove(request)
    }

    pub fn create_backup(
        &self,
        request: super::CreateBackupAppRequest,
    ) -> AppResult<super::CreatedBackupResult> {
        self.backups().create(request)
    }

    pub fn list_backups(
        &self,
        request: super::ListBackupsRequest,
    ) -> AppResult<super::BackupCatalogResult> {
        self.backups().list(request)
    }

    pub fn restore_backup(
        &self,
        request: super::RestoreBackupAppRequest,
    ) -> AppResult<super::TaskRun<super::RestoredBackupResult>> {
        self.backups().restore_collecting_progress(request)
    }

    pub fn inspect_bundle(
        &self,
        request: super::InspectBundleRequest,
    ) -> AppResult<super::BundleInspectionResult> {
        self.bundles().inspect(request)
    }

    pub fn pack_bundle(
        &self,
        request: super::PackBundleAppRequest,
    ) -> AppResult<super::CreatedBundleResult> {
        self.bundles().pack(request)
    }

    pub fn plan_bundle_apply(
        &self,
        request: super::PlanBundleApplyRequest,
    ) -> AppResult<super::BundleApplyPlanResult> {
        self.bundles().plan_apply(request)
    }

    pub fn apply_bundle(
        &self,
        request: super::ApplyBundleAppRequest,
    ) -> AppResult<super::TaskRun<super::BundleApplyResult>> {
        self.bundles().apply_collecting_progress(request)
    }

    pub fn inspect_config(
        &self,
        request: super::InspectConfigAppRequest,
    ) -> AppResult<super::TaskRun<super::ConfigInspectionResult>> {
        self.configs().inspect_collecting_progress(request)
    }

    pub fn plan_config_apply(
        &self,
        request: super::PlanConfigApplyAppRequest,
    ) -> AppResult<super::TaskRun<super::ConfigApplyPlanResult>> {
        self.configs().plan_apply_collecting_progress(request)
    }

    pub fn apply_config(
        &self,
        request: super::ApplyConfigAppRequest,
    ) -> AppResult<super::TaskRun<super::ConfigApplyResult>> {
        self.configs().apply_collecting_progress(request)
    }

    pub fn analyze_external_package(
        &self,
        request: super::AnalyzeExternalPackageAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageAnalysisResult>> {
        self.external_packages()
            .analyze_collecting_progress(request)
    }

    pub fn create_external_package_bundle(
        &self,
        request: super::CreateExternalPackageBundleAppRequest,
    ) -> AppResult<super::ExternalPackageBundleHandle> {
        self.external_packages().create_bundle(request)
    }

    pub fn plan_external_package_apply(
        &self,
        request: super::PlanExternalPackageApplyAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageApplyPlanResult>> {
        self.external_packages()
            .plan_apply_collecting_progress(request)
    }

    pub fn apply_external_package(
        &self,
        request: super::ApplyExternalPackageAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageApplyResult>> {
        self.external_packages().apply_collecting_progress(request)
    }

    pub(super) fn installations(&self) -> &InstallationService {
        &self.installations
    }

    pub(super) fn addons(&self) -> &AddonService {
        &self.addons
    }

    pub(super) fn addon_policies(&self) -> &AddonPolicyService {
        &self.addon_policies
    }

    pub(super) fn backups(&self) -> &BackupService {
        &self.backups
    }

    pub(super) fn bundles(&self) -> &BundleService {
        &self.bundles
    }

    pub(super) fn configs(&self) -> &ConfigService {
        &self.configs
    }

    pub(super) fn external_packages(&self) -> &ExternalPackageService {
        &self.external_packages
    }

    pub(super) fn runtime_settings(&self) -> &RuntimeSettingsService {
        &self.runtime_settings
    }
}
#[cfg(test)]
mod tests;
