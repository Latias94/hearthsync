use crate::core::addon::lock::{
    apply_addon_lock_sync_task_with_provider, diff_addon_locks, inspect_addon_lock,
    plan_addon_lock_sync, verify_addon_lock, write_addon_lock,
};
use crate::core::app::{
    AddonLockApplyResult, AddonLockDiffResult, AddonLockInspectionResult, AddonLockPlanResult,
    AddonLockVerifyResult, AddonLockWriteResult, AppRuntime, ApplyAddonLockAppRequest,
    CancellationToken, DiffAddonLockRequest, InspectAddonLockRequest, PlanAddonLockSyncRequest,
    TaskProgressEvent, TaskProgressSink, TaskRun, VerifyAddonLockRequest, WriteAddonLockRequest,
    task_support,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct AddonLockService {
    runtime: AppRuntime,
}

impl AddonLockService {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    #[cfg(test)]
    pub(crate) fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn inspect(
        &self,
        request: InspectAddonLockRequest,
    ) -> AppResult<AddonLockInspectionResult> {
        let installation = request.into_domain_installation();
        let inspection = inspect_addon_lock(&installation)?;
        Ok(AddonLockInspectionResult::from_domain(inspection))
    }

    pub fn write(&self, request: WriteAddonLockRequest) -> AppResult<AddonLockWriteResult> {
        let installation = request.into_domain_installation();
        let written = write_addon_lock(&installation)?;
        Ok(AddonLockWriteResult::from_domain(written))
    }

    pub fn diff(&self, request: DiffAddonLockRequest) -> AppResult<AddonLockDiffResult> {
        let diff = diff_addon_locks(&request.left_lock_path, &request.right_lock_path)?;
        Ok(AddonLockDiffResult::from_domain(diff))
    }

    pub fn verify(&self, request: VerifyAddonLockRequest) -> AppResult<AddonLockVerifyResult> {
        let (installation, lock_path) = request.into_domain_inputs();
        let verification = verify_addon_lock(&installation, lock_path.as_deref())?;
        Ok(AddonLockVerifyResult::from_domain(verification))
    }

    pub fn plan_sync(&self, request: PlanAddonLockSyncRequest) -> AppResult<AddonLockPlanResult> {
        let (installation, lock_path) = request.into_domain_inputs();
        let plan = plan_addon_lock_sync(&installation, lock_path.as_deref())?;
        Ok(AddonLockPlanResult::from_domain(plan))
    }

    pub fn apply_sync(&self, request: ApplyAddonLockAppRequest) -> AppResult<AddonLockApplyResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.apply_sync_task(request, cancellation, progress)
        })
    }

    pub fn apply_sync_task<TCancel, TProgress>(
        &self,
        request: ApplyAddonLockAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonLockApplyResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let applied = apply_addon_lock_sync_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(AddonLockApplyResult::from_domain(applied))
    }

    pub fn apply_sync_collecting_progress(
        &self,
        request: ApplyAddonLockAppRequest,
    ) -> AppResult<TaskRun<AddonLockApplyResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.apply_sync_task(request, cancellation, progress)
        })
    }

    pub fn apply_sync_with_callbacks<FCancel, FProgress>(
        &self,
        request: ApplyAddonLockAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonLockApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.apply_sync_task(request, cancellation, progress)
        })
    }
}
#[cfg(test)]
mod tests;
