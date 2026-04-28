use crate::core::addon::{
    adopt_addons, install_addon_task_with_provider, list_addons, relink_addon_with_provider,
    remove_addons_task, search_addons_with_provider, update_addons_task_with_provider,
    validate_addon_update_dependency_policy_support,
};
use crate::core::app::{
    AddonCachePurgeResult, AddonCacheRepairResult, AddonInventoryResult, AddonSearchCatalogResult,
    AdoptAddonsAppRequest, AdoptedAddonPackageResult, AppRuntime, CancellationToken,
    InstallAddonAppRequest, InstalledAddonPackageResult, ListAddonsRequest, RelinkAddonAppRequest,
    RelinkedAddonPackageResult, RemoveAddonAppRequest, RemovedAddonPackageResult,
    SearchAddonsRequest, TaskProgressEvent, TaskProgressSink, TaskRun, UpdateAddonAppRequest,
    UpdatedAddonPackageResult, task_support,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub(super) struct AddonService {
    runtime: AppRuntime,
}

impl AddonService {
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

    pub(super) fn search(
        &self,
        request: SearchAddonsRequest,
    ) -> AppResult<AddonSearchCatalogResult> {
        let results = search_addons_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request()?,
        )?;
        Ok(AddonSearchCatalogResult::from_domain_with_provider(
            results,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn list(&self, request: ListAddonsRequest) -> AppResult<AddonInventoryResult> {
        let installation = request.into_domain_installation()?;
        let state_paths = self.runtime.addon_state_paths(&installation)?;
        let inventory = list_addons(&installation, &state_paths)?;
        Ok(AddonInventoryResult::from_domain_with_provider(
            inventory,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn adopt(
        &self,
        request: AdoptAddonsAppRequest,
    ) -> AppResult<AdoptedAddonPackageResult> {
        let adopted = adopt_addons(request.into_domain_request(&self.runtime)?)?;
        Ok(AdoptedAddonPackageResult::from_domain_with_provider(
            adopted,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn relink(
        &self,
        request: RelinkAddonAppRequest,
    ) -> AppResult<RelinkedAddonPackageResult> {
        let relinked = relink_addon_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime)?,
        )?;
        Ok(RelinkedAddonPackageResult::from_domain_with_provider(
            relinked,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn purge_cache(&self) -> AppResult<AddonCachePurgeResult> {
        self.runtime
            .addon_provider()
            .purge_download_cache()
            .map(AddonCachePurgeResult::from_domain)
    }

    pub(super) fn repair_cache(&self) -> AppResult<AddonCacheRepairResult> {
        self.runtime
            .addon_provider()
            .repair_download_cache()
            .map(AddonCacheRepairResult::from_domain)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn install(
        &self,
        request: InstallAddonAppRequest,
    ) -> AppResult<InstalledAddonPackageResult> {
        task_support::run_service_task_direct(self, request, Self::install_task)
    }

    pub(super) fn install_task<TCancel, TProgress>(
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
            request.into_domain_request(&self.runtime)?,
            cancellation,
            progress,
        )?;
        Ok(InstalledAddonPackageResult::from_domain_with_provider(
            installed,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn install_collecting_progress(
        &self,
        request: InstallAddonAppRequest,
    ) -> AppResult<TaskRun<InstalledAddonPackageResult>> {
        task_support::run_service_task_collecting(self, request, Self::install_task)
    }

    #[allow(dead_code)]
    pub(super) fn install_with_callbacks<FCancel, FProgress>(
        &self,
        request: InstallAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<InstalledAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::install_task,
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn update(
        &self,
        request: UpdateAddonAppRequest,
    ) -> AppResult<UpdatedAddonPackageResult> {
        task_support::run_service_task_direct(self, request, Self::update_task)
    }

    pub(super) fn update_task<TCancel, TProgress>(
        &self,
        request: UpdateAddonAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<UpdatedAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let installation = request.installation.clone().into_domain()?;
        let state_paths = self.runtime.addon_state_paths(&installation)?;
        validate_addon_update_dependency_policy_support(
            self.runtime.addon_provider(),
            &installation,
            &state_paths,
            request.name.as_deref(),
        )?;
        let updated = update_addons_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime)?,
            cancellation,
            progress,
        )?;
        Ok(UpdatedAddonPackageResult::from_domain_with_provider(
            updated,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn update_collecting_progress(
        &self,
        request: UpdateAddonAppRequest,
    ) -> AppResult<TaskRun<UpdatedAddonPackageResult>> {
        task_support::run_service_task_collecting(self, request, Self::update_task)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn update_with_callbacks<FCancel, FProgress>(
        &self,
        request: UpdateAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<UpdatedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::update_task,
        )
    }

    #[allow(dead_code)]
    pub(super) fn remove(
        &self,
        request: RemoveAddonAppRequest,
    ) -> AppResult<RemovedAddonPackageResult> {
        task_support::run_service_task_direct(self, request, Self::remove_task)
    }

    pub(super) fn remove_task<TCancel, TProgress>(
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
            request.into_domain_request(&self.runtime)?,
            cancellation,
            progress,
        )?;
        Ok(RemovedAddonPackageResult::from_domain_with_provider(
            removed,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn remove_collecting_progress(
        &self,
        request: RemoveAddonAppRequest,
    ) -> AppResult<TaskRun<RemovedAddonPackageResult>> {
        task_support::run_service_task_collecting(self, request, Self::remove_task)
    }

    #[allow(dead_code)]
    pub(super) fn remove_with_callbacks<FCancel, FProgress>(
        &self,
        request: RemoveAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<RemovedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::remove_task,
        )
    }
}
#[cfg(test)]
mod tests;
