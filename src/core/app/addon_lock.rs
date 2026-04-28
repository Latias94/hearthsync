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
pub(super) struct AddonLockService {
    runtime: AppRuntime,
}

impl AddonLockService {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    #[cfg(test)]
    pub(super) fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub(super) fn inspect(
        &self,
        request: InspectAddonLockRequest,
    ) -> AppResult<AddonLockInspectionResult> {
        let (installation, state_paths) = request.into_domain_inputs(&self.runtime)?;
        let inspection = inspect_addon_lock(&installation, &state_paths)?;
        Ok(AddonLockInspectionResult::from_domain_with_provider(
            inspection,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn write(&self, request: WriteAddonLockRequest) -> AppResult<AddonLockWriteResult> {
        let (installation, state_paths) = request.into_domain_inputs(&self.runtime)?;
        let written = write_addon_lock(&installation, &state_paths)?;
        Ok(AddonLockWriteResult::from_domain(written))
    }

    pub(super) fn diff(&self, request: DiffAddonLockRequest) -> AppResult<AddonLockDiffResult> {
        let diff = diff_addon_locks(&request.left_lock_path, &request.right_lock_path)?;
        Ok(AddonLockDiffResult::from_domain_with_provider(
            diff,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn verify(
        &self,
        request: VerifyAddonLockRequest,
    ) -> AppResult<AddonLockVerifyResult> {
        let (installation, state_paths, lock_path) = request.into_domain_inputs(&self.runtime)?;
        let verification = verify_addon_lock(&installation, &state_paths, lock_path.as_deref())?;
        Ok(AddonLockVerifyResult::from_domain_with_provider(
            verification,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn plan_sync(
        &self,
        request: PlanAddonLockSyncRequest,
    ) -> AppResult<AddonLockPlanResult> {
        let (installation, state_paths, lock_path) = request.into_domain_inputs(&self.runtime)?;
        let plan = plan_addon_lock_sync(&installation, &state_paths, lock_path.as_deref())?;
        Ok(AddonLockPlanResult::from_domain_with_provider(
            plan,
            self.runtime.addon_provider(),
        ))
    }

    #[allow(dead_code)]
    pub(super) fn apply_sync(
        &self,
        request: ApplyAddonLockAppRequest,
    ) -> AppResult<AddonLockApplyResult> {
        task_support::run_service_task_direct(self, request, Self::apply_sync_task)
    }

    pub(super) fn apply_sync_task<TCancel, TProgress>(
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
            request.into_domain_request(&self.runtime)?,
            cancellation,
            progress,
        )?;
        Ok(AddonLockApplyResult::from_domain_with_provider(
            applied,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn apply_sync_collecting_progress(
        &self,
        request: ApplyAddonLockAppRequest,
    ) -> AppResult<TaskRun<AddonLockApplyResult>> {
        task_support::run_service_task_collecting(self, request, Self::apply_sync_task)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn apply_sync_with_callbacks<FCancel, FProgress>(
        &self,
        request: ApplyAddonLockAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonLockApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::apply_sync_task,
        )
    }
}
#[cfg(test)]
mod tests;
