use super::{
    AddonIndexService, AddonLockService, AddonService, AppRuntime, AppRuntimeCapabilitiesValue,
    BackupService, BundleService, ExternalPackageService, InstallationService, StableAppServices,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct HearthSyncApp {
    runtime: AppRuntime,
}

impl HearthSyncApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn capabilities(&self) -> AppRuntimeCapabilitiesValue {
        self.stable_services().capabilities()
    }

    pub fn scan_installations(&self) -> AppResult<super::InstallationScanResult> {
        self.stable_services().scan_installations()
    }

    pub fn inspect_installation(
        &self,
        request: super::InspectInstallationRequest,
    ) -> AppResult<super::InstallationInspectionResult> {
        self.stable_services().inspect_installation(request)
    }

    pub fn resolve_installation(
        &self,
        request: super::ResolveInstallationRequest,
    ) -> AppResult<super::ResolvedInstallationValue> {
        self.stable_services().resolve_installation(request)
    }

    pub fn search_addons(
        &self,
        request: super::SearchAddonsRequest,
    ) -> AppResult<super::AddonSearchCatalogResult> {
        self.stable_services().search_addons(request)
    }

    pub fn list_addons(
        &self,
        request: super::ListAddonsRequest,
    ) -> AppResult<super::AddonInventoryResult> {
        self.stable_services().list_addons(request)
    }

    pub fn install_addon(
        &self,
        request: super::InstallAddonAppRequest,
    ) -> AppResult<super::InstalledAddonPackageResult> {
        self.stable_services().install_addon(request)
    }

    pub fn install_addon_collecting_progress(
        &self,
        request: super::InstallAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::InstalledAddonPackageResult>> {
        self.stable_services()
            .install_addon_collecting_progress(request)
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
        self.stable_services()
            .install_addon_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn update_addons(
        &self,
        request: super::UpdateAddonAppRequest,
    ) -> AppResult<super::UpdatedAddonPackageResult> {
        self.stable_services().update_addons(request)
    }

    pub fn update_addons_collecting_progress(
        &self,
        request: super::UpdateAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::UpdatedAddonPackageResult>> {
        self.stable_services()
            .update_addons_collecting_progress(request)
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
        self.stable_services()
            .update_addons_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn remove_addons(
        &self,
        request: super::RemoveAddonAppRequest,
    ) -> AppResult<super::RemovedAddonPackageResult> {
        self.stable_services().remove_addons(request)
    }

    pub fn remove_addons_collecting_progress(
        &self,
        request: super::RemoveAddonAppRequest,
    ) -> AppResult<super::TaskRun<super::RemovedAddonPackageResult>> {
        self.stable_services()
            .remove_addons_collecting_progress(request)
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
        self.stable_services()
            .remove_addons_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn inspect_addon_index(
        &self,
        request: super::InspectAddonIndexRequest,
    ) -> AppResult<super::AddonIndexInspectionResult> {
        self.addon_indexes().inspect(request)
    }

    pub fn install_addon_index(
        &self,
        request: super::InstallAddonIndexAppRequest,
    ) -> AppResult<super::AddonIndexInstallResult> {
        self.addon_indexes().install(request)
    }

    pub fn install_addon_index_collecting_progress(
        &self,
        request: super::InstallAddonIndexAppRequest,
    ) -> AppResult<super::TaskRun<super::AddonIndexInstallResult>> {
        self.addon_indexes().install_collecting_progress(request)
    }

    pub fn install_addon_index_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::InstallAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::AddonIndexInstallResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.addon_indexes()
            .install_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn update_addon_index(
        &self,
        request: super::UpdateAddonIndexAppRequest,
    ) -> AppResult<super::AddonIndexUpdateResult> {
        self.addon_indexes().update(request)
    }

    pub fn update_addon_index_collecting_progress(
        &self,
        request: super::UpdateAddonIndexAppRequest,
    ) -> AppResult<super::TaskRun<super::AddonIndexUpdateResult>> {
        self.addon_indexes().update_collecting_progress(request)
    }

    pub fn update_addon_index_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::UpdateAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::AddonIndexUpdateResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.addon_indexes()
            .update_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn inspect_addon_lock(
        &self,
        request: super::InspectAddonLockRequest,
    ) -> AppResult<super::AddonLockInspectionResult> {
        self.addon_locks().inspect(request)
    }

    pub fn write_addon_lock(
        &self,
        request: super::WriteAddonLockRequest,
    ) -> AppResult<super::AddonLockWriteResult> {
        self.addon_locks().write(request)
    }

    pub fn diff_addon_locks(
        &self,
        request: super::DiffAddonLockRequest,
    ) -> AppResult<super::AddonLockDiffResult> {
        self.addon_locks().diff(request)
    }

    pub fn verify_addon_lock(
        &self,
        request: super::VerifyAddonLockRequest,
    ) -> AppResult<super::AddonLockVerifyResult> {
        self.addon_locks().verify(request)
    }

    pub fn plan_addon_lock_sync(
        &self,
        request: super::PlanAddonLockSyncRequest,
    ) -> AppResult<super::AddonLockPlanResult> {
        self.addon_locks().plan_sync(request)
    }

    pub fn apply_addon_lock_sync(
        &self,
        request: super::ApplyAddonLockAppRequest,
    ) -> AppResult<super::AddonLockApplyResult> {
        self.addon_locks().apply_sync(request)
    }

    pub fn apply_addon_lock_sync_collecting_progress(
        &self,
        request: super::ApplyAddonLockAppRequest,
    ) -> AppResult<super::TaskRun<super::AddonLockApplyResult>> {
        self.addon_locks().apply_sync_collecting_progress(request)
    }

    pub fn apply_addon_lock_sync_with_callbacks<FCancel, FProgress>(
        &self,
        request: super::ApplyAddonLockAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<super::AddonLockApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(super::TaskProgressEvent),
    {
        self.addon_locks()
            .apply_sync_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn create_backup(
        &self,
        request: super::CreateBackupAppRequest,
    ) -> AppResult<super::CreatedBackupResult> {
        self.stable_services().create_backup(request)
    }

    pub fn list_backups(
        &self,
        request: super::ListBackupsRequest,
    ) -> AppResult<super::BackupCatalogResult> {
        self.stable_services().list_backups(request)
    }

    pub fn restore_backup(
        &self,
        request: super::RestoreBackupAppRequest,
    ) -> AppResult<super::RestoredBackupResult> {
        self.stable_services().restore_backup(request)
    }

    pub fn restore_backup_collecting_progress(
        &self,
        request: super::RestoreBackupAppRequest,
    ) -> AppResult<super::TaskRun<super::RestoredBackupResult>> {
        self.stable_services()
            .restore_backup_collecting_progress(request)
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
        self.stable_services()
            .restore_backup_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn inspect_bundle(
        &self,
        request: super::InspectBundleRequest,
    ) -> AppResult<super::BundleInspectionResult> {
        self.stable_services().inspect_bundle(request)
    }

    pub fn pack_bundle(
        &self,
        request: super::PackBundleAppRequest,
    ) -> AppResult<super::CreatedBundleResult> {
        self.stable_services().pack_bundle(request)
    }

    pub fn plan_bundle_apply(
        &self,
        request: super::PlanBundleApplyRequest,
    ) -> AppResult<super::BundleApplyPlanResult> {
        self.stable_services().plan_bundle_apply(request)
    }

    pub fn apply_bundle(
        &self,
        request: super::ApplyBundleAppRequest,
    ) -> AppResult<super::BundleApplyResult> {
        self.stable_services().apply_bundle(request)
    }

    pub fn apply_bundle_collecting_progress(
        &self,
        request: super::ApplyBundleAppRequest,
    ) -> AppResult<super::TaskRun<super::BundleApplyResult>> {
        self.stable_services()
            .apply_bundle_collecting_progress(request)
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
        self.stable_services()
            .apply_bundle_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn plan_bundle_addon_lock(
        &self,
        request: super::PlanBundleAddonLockRequest,
    ) -> AppResult<super::BundleAddonLockPlanResult> {
        self.bundles().plan_addon_lock(request)
    }

    pub fn apply_bundle_addon_lock(
        &self,
        request: super::ApplyBundleAddonLockAppRequest,
    ) -> AppResult<super::BundleAddonLockApplyResult> {
        self.bundles().apply_addon_lock(request)
    }

    pub fn analyze_external_package(
        &self,
        request: super::AnalyzeExternalPackageAppRequest,
    ) -> AppResult<super::ExternalPackageAnalysisResult> {
        self.stable_services().analyze_external_package(request)
    }

    pub fn analyze_external_package_collecting_progress(
        &self,
        request: super::AnalyzeExternalPackageAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageAnalysisResult>> {
        self.stable_services()
            .analyze_external_package_collecting_progress(request)
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
        self.stable_services()
            .analyze_external_package_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn create_external_package_bundle(
        &self,
        request: super::CreateExternalPackageBundleAppRequest,
    ) -> AppResult<super::ExternalPackageBundleHandle> {
        self.stable_services()
            .create_external_package_bundle(request)
    }

    pub fn plan_external_package_apply(
        &self,
        request: super::PlanExternalPackageApplyAppRequest,
    ) -> AppResult<super::ExternalPackageApplyPlanResult> {
        self.stable_services().plan_external_package_apply(request)
    }

    pub fn plan_external_package_apply_collecting_progress(
        &self,
        request: super::PlanExternalPackageApplyAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageApplyPlanResult>> {
        self.stable_services()
            .plan_external_package_apply_collecting_progress(request)
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
        self.stable_services()
            .plan_external_package_apply_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn apply_external_package(
        &self,
        request: super::ApplyExternalPackageAppRequest,
    ) -> AppResult<super::ExternalPackageApplyResult> {
        self.stable_services().apply_external_package(request)
    }

    pub fn apply_external_package_collecting_progress(
        &self,
        request: super::ApplyExternalPackageAppRequest,
    ) -> AppResult<super::TaskRun<super::ExternalPackageApplyResult>> {
        self.stable_services()
            .apply_external_package_collecting_progress(request)
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
        self.stable_services()
            .apply_external_package_with_callbacks(request, is_cancelled, on_progress)
    }

    pub fn stable_services(&self) -> StableAppServices {
        StableAppServices::with_runtime(self.runtime.clone())
    }

    pub fn installations(&self) -> InstallationService {
        InstallationService::with_runtime(self.runtime.clone())
    }

    pub fn addons(&self) -> AddonService {
        AddonService::with_runtime(self.runtime.clone())
    }

    pub fn addon_indexes(&self) -> AddonIndexService {
        AddonIndexService::with_runtime(self.runtime.clone())
    }

    pub fn addon_locks(&self) -> AddonLockService {
        AddonLockService::with_runtime(self.runtime.clone())
    }

    pub fn backups(&self) -> BackupService {
        BackupService::with_runtime(self.runtime.clone())
    }

    pub fn bundles(&self) -> BundleService {
        BundleService::with_runtime(self.runtime.clone())
    }

    pub fn external_packages(&self) -> ExternalPackageService {
        ExternalPackageService::with_runtime(self.runtime.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::core::app::{
        AddonProviderModeValue, AddonProviderOptionsValue, AddonProviderRetryPolicyValue,
        ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue,
        ExternalHelperPolicyValue, HealthStatusValue, HelperStrategyValue, HostPlatformValue,
        InspectInstallationRequest, ResolveInstallationRequest, WowFlavorValue,
    };

    #[test]
    fn hearthsync_app_builds_services_with_shared_runtime() {
        let temp = tempdir().expect("temp dir");
        let scan_root = temp.path().join("scan-root");
        let backup_dir = temp.path().join("backups");
        let bundle_dir = temp.path().join("bundles");
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_install_scan_roots(Some(vec![scan_root.clone()]))
            .with_default_backup_dir(Some(backup_dir.clone()))
            .with_default_bundle_output_dir(Some(bundle_dir.clone()));

        let app = HearthSyncApp::with_runtime(runtime);

        assert_eq!(
            app.installations().runtime().install_scan_roots(),
            Some([scan_root].as_slice())
        );
        assert_eq!(
            app.installations().runtime().host_platform(),
            HostPlatformValue::MacOs
        );
        assert_eq!(
            app.backups().runtime().default_backup_dir(),
            Some(backup_dir.as_path())
        );
        assert_eq!(
            app.bundles().runtime().default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(
            app.external_packages()
                .runtime()
                .default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(
            app.addons().runtime().host_platform(),
            HostPlatformValue::MacOs
        );
        assert_eq!(
            app.addon_indexes().runtime().host_platform(),
            HostPlatformValue::MacOs
        );
        assert_eq!(
            app.addon_locks().runtime().host_platform(),
            HostPlatformValue::MacOs
        );
    }

    #[test]
    fn hearthsync_app_exposes_first_wave_stable_services() {
        let temp = tempdir().expect("temp dir");
        let backup_dir = temp.path().join("backups");
        let bundle_dir = temp.path().join("bundles");
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_default_backup_dir(Some(backup_dir.clone()))
            .with_default_bundle_output_dir(Some(bundle_dir.clone()));

        let app = HearthSyncApp::with_runtime(runtime);
        let stable = app.stable_services();

        assert_eq!(stable.runtime().host_platform(), HostPlatformValue::MacOs);
        assert_eq!(
            stable.backups().runtime().default_backup_dir(),
            Some(backup_dir.as_path())
        );
        assert_eq!(
            stable.bundles().runtime().default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(
            stable.addons().runtime().host_platform(),
            HostPlatformValue::MacOs
        );
    }

    #[test]
    fn hearthsync_app_exposes_runtime_capabilities_as_app_owned_value() {
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_external_helper_policy(ExternalHelperPolicyValue::PreferExternal);
        let app = HearthSyncApp::with_runtime(runtime);

        assert_eq!(
            app.capabilities(),
            AppRuntimeCapabilitiesValue {
                addon_provider: AddonProviderModeValue::ConfiguredDefault {
                    options: AddonProviderOptionsValue {
                        download_cache_dir: None,
                        retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
                    },
                },
                external_helper: ExternalHelperCapabilitiesValue {
                    policy: ExternalHelperPolicyValue::PreferExternal,
                    availability: ExternalHelperAvailabilityValue::Unavailable,
                    active_strategy: HelperStrategyValue::NativeRust,
                },
            }
        );
    }

    #[test]
    fn hearthsync_app_direct_installation_entrypoints_use_shared_runtime() {
        let temp = tempdir().expect("temp dir");
        let product_root = temp.path().join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");

        fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
        fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");
        fs::write(
            flavor_root.join("WTF").join("Config.wtf"),
            "SET locale enUS",
        )
        .expect("config");

        let app = HearthSyncApp::with_runtime(
            AppRuntime::new()
                .with_host_platform(HostPlatformValue::MacOs)
                .with_install_scan_roots(Some(vec![product_root.clone()])),
        );

        let scanned = app.scan_installations().expect("scan installations");
        let inspected = app
            .inspect_installation(InspectInstallationRequest {
                path: product_root.clone(),
                flavor: Some(WowFlavorValue::Retail),
            })
            .expect("inspect installation");
        let resolved = app
            .resolve_installation(ResolveInstallationRequest {
                path: product_root,
                flavor: Some(WowFlavorValue::Retail),
            })
            .expect("resolve installation");

        assert_eq!(scanned.installation_count, 1);
        assert_eq!(scanned.installations[0].platform, HostPlatformValue::MacOs);
        assert_eq!(inspected.installation.platform, HostPlatformValue::MacOs);
        assert_eq!(inspected.health.status, HealthStatusValue::Warning);
        assert_eq!(resolved.platform, HostPlatformValue::MacOs);
        assert!(
            resolved
                .flavor_root
                .ends_with(Path::new("World of Warcraft").join("_retail_"))
        );
    }
}
