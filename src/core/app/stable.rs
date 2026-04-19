use super::{
    AddonService, AppRuntime, AppRuntimeCapabilitiesValue, BackupService, BundleService,
    ExternalPackageService, InstallationService,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct StableAppServices {
    pub(super) runtime: AppRuntime,
}

impl StableAppServices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    #[cfg(test)]
    pub(crate) fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn capabilities(&self) -> AppRuntimeCapabilitiesValue {
        self.runtime.capabilities()
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

    pub fn install_addon(
        &self,
        request: super::InstallAddonAppRequest,
    ) -> AppResult<super::InstalledAddonPackageResult> {
        self.addons().install(request)
    }

    pub fn install_addon_collecting_progress(
        &self,
        request: super::InstallAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::InstalledAddonPackageResult>> {
        self.addons().install_collecting_progress(request)
    }

    pub fn install_addon_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::InstallAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::InstalledAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.addons()
            .install_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn update_addons(
        &self,
        request: super::UpdateAddonAppRequest,
    ) -> AppResult<super::UpdatedAddonPackageResult> {
        self.addons().update(request)
    }

    pub fn update_addons_collecting_progress(
        &self,
        request: super::UpdateAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::UpdatedAddonPackageResult>> {
        self.addons().update_collecting_progress(request)
    }

    pub fn update_addons_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::UpdateAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::UpdatedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.addons()
            .update_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn remove_addons(
        &self,
        request: super::RemoveAddonAppRequest,
    ) -> AppResult<super::RemovedAddonPackageResult> {
        self.addons().remove(request)
    }

    pub fn remove_addons_collecting_progress(
        &self,
        request: super::RemoveAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::RemovedAddonPackageResult>> {
        self.addons().remove_collecting_progress(request)
    }

    pub fn remove_addons_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::RemoveAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::RemovedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.addons()
            .remove_with_callbacks(request, is_cancelled, on_progress)
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
    ) -> AppResult<super::RestoredBackupResult> {
        self.backups().restore(request)
    }

    pub fn restore_backup_collecting_progress(
        &self,
        request: super::RestoreBackupAppRequest,
    ) -> AppResult<super::TaskRun<super::RestoredBackupResult>> {
        self.backups().restore_collecting_progress(request)
    }

    pub fn restore_backup_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::RestoreBackupAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::RestoredBackupResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.backups()
            .restore_with_callbacks(request, is_cancelled, on_progress)
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
    ) -> AppResult<super::BundleApplyResult> {
        self.bundles().apply(request)
    }

    pub fn apply_bundle_collecting_progress(
        &self,
        request: super::ApplyBundleAppRequest,
    ) -> AppResult<super::TaskRun<super::BundleApplyResult>> {
        self.bundles().apply_collecting_progress(request)
    }

    pub fn apply_bundle_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::ApplyBundleAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::BundleApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.bundles()
            .apply_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn analyze_external_package(
        &self,
        request: super::AnalyzeExternalPackageAppRequest,
    ) -> AppResult<super::ExternalPackageAnalysisResult> {
        self.external_packages().analyze(request)
    }

    pub fn analyze_external_package_collecting_progress(
        &self,
        request: super::AnalyzeExternalPackageAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageAnalysisResult>> {
        self.external_packages()
            .analyze_collecting_progress(request)
    }

    pub fn analyze_external_package_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::AnalyzeExternalPackageAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::ExternalPackageAnalysisResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.external_packages()
            .analyze_with_callbacks(request, is_cancelled, on_progress)
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
    ) -> AppResult<super::ExternalPackageApplyPlanResult> {
        self.external_packages().plan_apply(request)
    }

    pub fn plan_external_package_apply_collecting_progress(
        &self,
        request: super::PlanExternalPackageApplyAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageApplyPlanResult>> {
        self.external_packages()
            .plan_apply_collecting_progress(request)
    }

    pub fn plan_external_package_apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::PlanExternalPackageApplyAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::ExternalPackageApplyPlanResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.external_packages()
            .plan_apply_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn apply_external_package(
        &self,
        request: super::ApplyExternalPackageAppRequest,
    ) -> AppResult<super::ExternalPackageApplyResult> {
        self.external_packages().apply(request)
    }

    pub fn apply_external_package_collecting_progress(
        &self,
        request: super::ApplyExternalPackageAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageApplyResult>> {
        self.external_packages().apply_collecting_progress(request)
    }

    pub fn apply_external_package_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::ApplyExternalPackageAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::ExternalPackageApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.external_packages()
            .apply_with_callbacks(request, is_cancelled, on_progress)
    }

    pub(crate) fn installations(&self) -> InstallationService {
        InstallationService::with_runtime(self.runtime.clone())
    }

    pub(crate) fn addons(&self) -> AddonService {
        AddonService::with_runtime(self.runtime.clone())
    }

    pub(crate) fn backups(&self) -> BackupService {
        BackupService::with_runtime(self.runtime.clone())
    }

    pub(crate) fn bundles(&self) -> BundleService {
        BundleService::with_runtime(self.runtime.clone())
    }

    pub(crate) fn external_packages(&self) -> ExternalPackageService {
        ExternalPackageService::with_runtime(self.runtime.clone())
    }
}
#[cfg(test)]
mod tests;
