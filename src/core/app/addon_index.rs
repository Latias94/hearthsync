use crate::core::addon::index::{
    inspect_addon_index, install_addon_from_index_task_with_provider,
    update_addons_from_index_task_with_provider,
};
use crate::core::app::{
    AddonIndexInspectionResult, AddonIndexInstallResult, AddonIndexUpdateResult, AppRuntime,
    CancellationToken, InspectAddonIndexRequest, InstallAddonIndexAppRequest, TaskProgressEvent,
    TaskProgressSink, TaskRun, UpdateAddonIndexAppRequest, task_support,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct AddonIndexService {
    runtime: AppRuntime,
}

impl AddonIndexService {
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
        request: InspectAddonIndexRequest,
    ) -> AppResult<AddonIndexInspectionResult> {
        let inspection = inspect_addon_index(&request.index_path)?;
        Ok(AddonIndexInspectionResult::from_domain(inspection))
    }

    pub fn install(
        &self,
        request: InstallAddonIndexAppRequest,
    ) -> AppResult<AddonIndexInstallResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_task<TCancel, TProgress>(
        &self,
        request: InstallAddonIndexAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonIndexInstallResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let installed = install_addon_from_index_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(AddonIndexInstallResult::from_domain(installed))
    }

    pub fn install_collecting_progress(
        &self,
        request: InstallAddonIndexAppRequest,
    ) -> AppResult<TaskRun<AddonIndexInstallResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_with_callbacks<FCancel, FProgress>(
        &self,
        request: InstallAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexInstallResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn update(&self, request: UpdateAddonIndexAppRequest) -> AppResult<AddonIndexUpdateResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_task<TCancel, TProgress>(
        &self,
        request: UpdateAddonIndexAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonIndexUpdateResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let updated = update_addons_from_index_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(AddonIndexUpdateResult::from_domain(updated))
    }

    pub fn update_collecting_progress(
        &self,
        request: UpdateAddonIndexAppRequest,
    ) -> AppResult<TaskRun<AddonIndexUpdateResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_with_callbacks<FCancel, FProgress>(
        &self,
        request: UpdateAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexUpdateResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }
}
#[cfg(test)]
mod tests;
