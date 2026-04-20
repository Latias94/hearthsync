use crate::core::app::{
    AnalyzeExternalPackageAppRequest, AppRuntime, ApplyExternalPackageAppRequest,
    CancellationToken, CreateExternalPackageBundleAppRequest, ExternalPackageAnalysisResult,
    ExternalPackageApplyPlanResult, ExternalPackageApplyResult, ExternalPackageBundleHandle,
    PlanExternalPackageApplyAppRequest, TaskProgressEvent, TaskProgressSink, TaskRun, task_support,
};
use crate::core::bundle::{
    analyze_external_package_task, apply_external_package_task, create_external_package_bundle,
    plan_external_package_apply_task,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub(super) struct ExternalPackageService {
    runtime: AppRuntime,
}

impl ExternalPackageService {
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

    pub(super) fn analyze(
        &self,
        request: AnalyzeExternalPackageAppRequest,
    ) -> AppResult<ExternalPackageAnalysisResult> {
        task_support::run_service_task_direct(self, request, Self::analyze_task)
    }

    pub(super) fn analyze_task<TCancel, TProgress>(
        &self,
        request: AnalyzeExternalPackageAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageAnalysisResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let analysis =
            analyze_external_package_task(request.into_domain_request(), cancellation, progress)?;
        Ok(ExternalPackageAnalysisResult::from_domain(analysis))
    }

    pub(super) fn analyze_collecting_progress(
        &self,
        request: AnalyzeExternalPackageAppRequest,
    ) -> AppResult<TaskRun<ExternalPackageAnalysisResult>> {
        task_support::run_service_task_collecting(self, request, Self::analyze_task)
    }

    pub(super) fn analyze_with_callbacks<FCancel, FProgress>(
        &self,
        request: AnalyzeExternalPackageAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageAnalysisResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::analyze_task,
        )
    }

    pub(super) fn create_bundle(
        &self,
        request: CreateExternalPackageBundleAppRequest,
    ) -> AppResult<ExternalPackageBundleHandle> {
        let bundle = create_external_package_bundle(request.into_domain_request(&self.runtime))?;
        Ok(ExternalPackageBundleHandle::from_domain(bundle))
    }

    pub(super) fn plan_apply(
        &self,
        request: PlanExternalPackageApplyAppRequest,
    ) -> AppResult<ExternalPackageApplyPlanResult> {
        task_support::run_service_task_direct(self, request, Self::plan_apply_task)
    }

    pub(super) fn plan_apply_task<TCancel, TProgress>(
        &self,
        request: PlanExternalPackageApplyAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageApplyPlanResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let plan = plan_external_package_apply_task(
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(ExternalPackageApplyPlanResult::from_domain_plan(
            plan,
            self.runtime.helper_strategy(),
        ))
    }

    pub(super) fn plan_apply_collecting_progress(
        &self,
        request: PlanExternalPackageApplyAppRequest,
    ) -> AppResult<TaskRun<ExternalPackageApplyPlanResult>> {
        task_support::run_service_task_collecting(self, request, Self::plan_apply_task)
    }

    pub(super) fn plan_apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: PlanExternalPackageApplyAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageApplyPlanResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::plan_apply_task,
        )
    }

    pub(super) fn apply(
        &self,
        request: ApplyExternalPackageAppRequest,
    ) -> AppResult<ExternalPackageApplyResult> {
        task_support::run_service_task_direct(self, request, Self::apply_task)
    }

    pub(super) fn apply_task<TCancel, TProgress>(
        &self,
        request: ApplyExternalPackageAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageApplyResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let applied = apply_external_package_task(
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(ExternalPackageApplyResult::from_domain(applied))
    }

    pub(super) fn apply_collecting_progress(
        &self,
        request: ApplyExternalPackageAppRequest,
    ) -> AppResult<TaskRun<ExternalPackageApplyResult>> {
        task_support::run_service_task_collecting(self, request, Self::apply_task)
    }

    pub(super) fn apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: ApplyExternalPackageAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageApplyResult>
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
