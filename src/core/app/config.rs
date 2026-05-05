use crate::core::app::{
    ConfigApplyPlanResult, ConfigApplyResult, ConfigBundleHandle, ConfigInspectionResult,
    ExportConfigBundleAppRequest, ExternalPackageService, InspectConfigAppRequest,
    PlanConfigApplyAppRequest, TaskProgressEvent, TaskRun,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub(super) struct ConfigService {
    external_packages: ExternalPackageService,
}

impl ConfigService {
    pub(super) fn with_external_packages(external_packages: ExternalPackageService) -> Self {
        Self { external_packages }
    }

    pub(super) fn inspect_collecting_progress(
        &self,
        request: InspectConfigAppRequest,
    ) -> AppResult<TaskRun<ConfigInspectionResult>> {
        self.external_packages
            .analyze_collecting_progress(request.into_external_request())
            .map(|run| TaskRun {
                task_id: run.task_id,
                result: ConfigInspectionResult::from_external(run.result),
                progress: run.progress,
            })
    }

    pub(super) fn inspect_with_callbacks<FCancel, FProgress>(
        &self,
        request: InspectConfigAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ConfigInspectionResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        self.external_packages
            .analyze_with_callbacks(request.into_external_request(), is_cancelled, on_progress)
            .map(ConfigInspectionResult::from_external)
    }

    pub(super) fn create_bundle(
        &self,
        request: ExportConfigBundleAppRequest,
    ) -> AppResult<ConfigBundleHandle> {
        let handle = self
            .external_packages
            .create_bundle(request.into_external_request())?;
        Ok(ConfigBundleHandle::from_external(handle))
    }

    pub(super) fn plan_apply_collecting_progress(
        &self,
        request: PlanConfigApplyAppRequest,
    ) -> AppResult<TaskRun<ConfigApplyPlanResult>> {
        self.external_packages
            .plan_apply_collecting_progress(request.into_external_request())
            .map(|run| TaskRun {
                task_id: run.task_id,
                result: ConfigApplyPlanResult::from_external(run.result),
                progress: run.progress,
            })
    }

    pub(super) fn plan_apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: PlanConfigApplyAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ConfigApplyPlanResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        self.external_packages
            .plan_apply_with_callbacks(request.into_external_request(), is_cancelled, on_progress)
            .map(ConfigApplyPlanResult::from_external)
    }

    pub(super) fn apply_collecting_progress(
        &self,
        request: crate::core::app::ApplyConfigAppRequest,
    ) -> AppResult<TaskRun<ConfigApplyResult>> {
        self.external_packages
            .apply_collecting_progress(request.into_external_request())
            .map(|run| TaskRun {
                task_id: run.task_id,
                result: ConfigApplyResult::from_external(run.result),
                progress: run.progress,
            })
    }

    pub(super) fn apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: crate::core::app::ApplyConfigAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ConfigApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        self.external_packages
            .apply_with_callbacks(request.into_external_request(), is_cancelled, on_progress)
            .map(ConfigApplyResult::from_external)
    }
}

#[cfg(test)]
mod tests;
