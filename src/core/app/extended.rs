use super::{AddonIndexService, AddonLockService, AppRuntime, StableAppServices};
use crate::core::error::AppResult;

#[derive(Debug, Clone)]
pub struct ExtendedAppServices {
    stable: StableAppServices,
    addon_indexes: AddonIndexService,
    addon_locks: AddonLockService,
}

impl Default for ExtendedAppServices {
    fn default() -> Self {
        Self::with_runtime(AppRuntime::default())
    }
}

impl ExtendedAppServices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self {
            stable: StableAppServices::with_runtime(runtime.clone()),
            addon_indexes: AddonIndexService::with_runtime(runtime.clone()),
            addon_locks: AddonLockService::with_runtime(runtime),
        }
    }

    pub fn stable(&self) -> &StableAppServices {
        &self.stable
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

    pub fn plan_bundle_addon_lock(
        &self,
        request: super::PlanBundleAddonLockRequest,
    ) -> AppResult<super::BundleAddonLockPlanResult> {
        self.stable.bundles().plan_addon_lock(request)
    }

    pub fn apply_bundle_addon_lock(
        &self,
        request: super::ApplyBundleAddonLockAppRequest,
    ) -> AppResult<super::BundleAddonLockApplyResult> {
        self.stable.bundles().apply_addon_lock(request)
    }

    pub(super) fn addon_indexes(&self) -> &AddonIndexService {
        &self.addon_indexes
    }

    pub(super) fn addon_locks(&self) -> &AddonLockService {
        &self.addon_locks
    }
}
#[cfg(test)]
mod tests;
