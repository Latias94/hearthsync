use crate::core::app::{
    AppRuntime, ApplyBundleAddonLockAppRequest, ApplyBundleAppRequest, BundleAddonLockApplyResult,
    BundleAddonLockPlanResult, BundleApplyPlanResult, BundleApplyResult, BundleInspectionResult,
    CancellationToken, CreatedBundleResult, InspectBundleRequest, PackBundleAppRequest,
    PlanBundleAddonLockRequest, PlanBundleApplyRequest, TaskProgressEvent, TaskProgressSink,
    TaskRun, task_support,
};
use crate::core::bundle::{
    apply_bundle_addon_lock, inspect_bundle, pack_bundle, plan_bundle_addon_lock,
    plan_bundle_apply, unpack_bundle_task,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub(super) struct BundleService {
    runtime: AppRuntime,
}

impl BundleService {
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
        request: InspectBundleRequest,
    ) -> AppResult<BundleInspectionResult> {
        let inspection = inspect_bundle(&request.bundle_path)?;
        Ok(BundleInspectionResult::from_domain(inspection))
    }

    pub(super) fn pack(&self, request: PackBundleAppRequest) -> AppResult<CreatedBundleResult> {
        let bundle = pack_bundle(request.into_domain_request(&self.runtime))?;
        Ok(CreatedBundleResult::from_domain(bundle))
    }

    pub(super) fn plan_apply(
        &self,
        request: PlanBundleApplyRequest,
    ) -> AppResult<BundleApplyPlanResult> {
        let (bundle_path, installation, apply_mappings) = request.into_domain_inputs();
        let plan = plan_bundle_apply(&bundle_path, &installation, &apply_mappings)?;
        Ok(BundleApplyPlanResult::from_domain_plan(
            plan,
            self.runtime.helper_strategy(),
        ))
    }

    pub(super) fn apply(&self, request: ApplyBundleAppRequest) -> AppResult<BundleApplyResult> {
        task_support::run_service_task_direct(self, request, Self::apply_task)
    }

    pub(super) fn plan_addon_lock(
        &self,
        request: PlanBundleAddonLockRequest,
    ) -> AppResult<BundleAddonLockPlanResult> {
        let (bundle_path, installation) = request.into_domain_inputs();
        let plan = plan_bundle_addon_lock(&bundle_path, &installation)?;
        Ok(BundleAddonLockPlanResult::from_domain(plan))
    }

    pub(super) fn apply_addon_lock(
        &self,
        request: ApplyBundleAddonLockAppRequest,
    ) -> AppResult<BundleAddonLockApplyResult> {
        let applied = apply_bundle_addon_lock(request.into_domain_request(&self.runtime))?;
        Ok(BundleAddonLockApplyResult::from_domain(applied))
    }

    pub(super) fn apply_task<TCancel, TProgress>(
        &self,
        request: ApplyBundleAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<BundleApplyResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let applied = unpack_bundle_task(
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(BundleApplyResult::from_domain(applied))
    }

    pub(super) fn apply_collecting_progress(
        &self,
        request: ApplyBundleAppRequest,
    ) -> AppResult<TaskRun<BundleApplyResult>> {
        task_support::run_service_task_collecting(self, request, Self::apply_task)
    }

    pub(super) fn apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: ApplyBundleAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<BundleApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::apply_task,
        )
    }
}
#[cfg(test)]
mod tests;
