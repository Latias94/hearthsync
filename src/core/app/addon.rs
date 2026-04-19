use crate::core::addon::{
    install_addon_task_with_provider, list_addons, remove_addons_task, search_addons_with_provider,
    update_addons_task_with_provider,
};
use crate::core::app::{
    AddonInventoryResult, AddonSearchCatalogResult, AppRuntime, CancellationToken,
    InstallAddonAppRequest, InstalledAddonPackageResult, ListAddonsRequest, RemoveAddonAppRequest,
    RemovedAddonPackageResult, SearchAddonsRequest, TaskProgressEvent, TaskProgressSink, TaskRun,
    UpdateAddonAppRequest, UpdatedAddonPackageResult, task_support,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct AddonService {
    runtime: AppRuntime,
}

impl AddonService {
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

    pub fn search(&self, request: SearchAddonsRequest) -> AppResult<AddonSearchCatalogResult> {
        let results = search_addons_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(),
        )?;
        Ok(AddonSearchCatalogResult::from_domain(results))
    }

    pub fn list(&self, request: ListAddonsRequest) -> AppResult<AddonInventoryResult> {
        let installation = request.into_domain_installation();
        let inventory = list_addons(&installation)?;
        Ok(AddonInventoryResult::from_domain(inventory))
    }

    pub fn install(
        &self,
        request: InstallAddonAppRequest,
    ) -> AppResult<InstalledAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_task<TCancel, TProgress>(
        &self,
        request: InstallAddonAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<InstalledAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let installed = install_addon_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(InstalledAddonPackageResult::from_domain(installed))
    }

    pub fn install_collecting_progress(
        &self,
        request: InstallAddonAppRequest,
    ) -> AppResult<TaskRun<InstalledAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_with_callbacks<FCancel, FProgress>(
        &self,
        request: InstallAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<InstalledAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn update(&self, request: UpdateAddonAppRequest) -> AppResult<UpdatedAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_task<TCancel, TProgress>(
        &self,
        request: UpdateAddonAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<UpdatedAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let updated = update_addons_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(UpdatedAddonPackageResult::from_domain(updated))
    }

    pub fn update_collecting_progress(
        &self,
        request: UpdateAddonAppRequest,
    ) -> AppResult<TaskRun<UpdatedAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_with_callbacks<FCancel, FProgress>(
        &self,
        request: UpdateAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<UpdatedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn remove(&self, request: RemoveAddonAppRequest) -> AppResult<RemovedAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }

    pub fn remove_task<TCancel, TProgress>(
        &self,
        request: RemoveAddonAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<RemovedAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let removed = remove_addons_task(
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(RemovedAddonPackageResult::from_domain(removed))
    }

    pub fn remove_collecting_progress(
        &self,
        request: RemoveAddonAppRequest,
    ) -> AppResult<TaskRun<RemovedAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }

    pub fn remove_with_callbacks<FCancel, FProgress>(
        &self,
        request: RemoveAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<RemovedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }
}
#[cfg(test)]
mod tests;
