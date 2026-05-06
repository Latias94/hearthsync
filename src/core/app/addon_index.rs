use crate::core::addon::index::{
    attach_addons_from_index_task_with_provider, inspect_addon_index,
    install_addon_from_index_task_with_provider, relink_addon_from_index_task_with_provider,
    scaffold_addon_index, search_addon_index, search_community_addon_index,
    suggest_addon_index_hints, update_addons_from_index_task_with_provider,
    validate_addon_index_update_dependency_policy_support,
};
use crate::core::app::{
    AddonIndexAttachResult, AddonIndexInspectionResult, AddonIndexInstallResult,
    AddonIndexRelinkResult, AddonIndexScaffoldResult, AddonIndexSearchResult,
    AddonIndexSuggestionResult, AddonIndexUpdateResult, AddonIndexValidationResult, AppRuntime,
    AttachAddonIndexAppRequest, CancellationToken, InspectAddonIndexRequest,
    InstallAddonIndexAppRequest, RelinkAddonIndexAppRequest, ScaffoldAddonIndexRequest,
    SearchAddonIndexRequest, SuggestAddonIndexRequest, TaskProgressEvent, TaskProgressSink,
    TaskRun, UpdateAddonIndexAppRequest, task_support,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub(super) struct AddonIndexService {
    runtime: AppRuntime,
}

impl AddonIndexService {
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
        request: InspectAddonIndexRequest,
    ) -> AppResult<AddonIndexInspectionResult> {
        let index_path = request.into_index_path(&self.runtime)?;
        let inspection = inspect_addon_index(&index_path)?;
        Ok(AddonIndexInspectionResult::from_domain_with_provider(
            inspection,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn validate(
        &self,
        request: InspectAddonIndexRequest,
    ) -> AppResult<AddonIndexValidationResult> {
        let inspection = self.inspect(request)?;
        Ok(AddonIndexValidationResult::from_inspection(inspection))
    }

    pub(super) fn search(
        &self,
        request: SearchAddonIndexRequest,
    ) -> AppResult<AddonIndexSearchResult> {
        let search = search_addon_index(request.into_domain(&self.runtime)?)?;
        Ok(AddonIndexSearchResult::from_domain_with_provider(
            search,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn search_community(
        &self,
        query: String,
        limit: usize,
        installation: crate::core::install::DetectedFlavorInstallation,
    ) -> AppResult<AddonIndexSearchResult> {
        let search = search_community_addon_index(query, limit, installation.flavor)?;
        Ok(AddonIndexSearchResult::from_domain_with_provider(
            search,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn suggest(
        &self,
        request: SuggestAddonIndexRequest,
    ) -> AppResult<AddonIndexSuggestionResult> {
        let suggestion = suggest_addon_index_hints(request.into_domain(&self.runtime)?)?;
        Ok(AddonIndexSuggestionResult::from_domain(suggestion))
    }

    pub(super) fn scaffold(
        &self,
        request: ScaffoldAddonIndexRequest,
    ) -> AppResult<AddonIndexScaffoldResult> {
        let result = scaffold_addon_index(request.into_domain(&self.runtime)?)?;
        Ok(AddonIndexScaffoldResult::from_domain(result))
    }

    pub(super) fn attach_task<TCancel, TProgress>(
        &self,
        request: AttachAddonIndexAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonIndexAttachResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let attached = attach_addons_from_index_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime)?,
            cancellation,
            progress,
        )?;
        Ok(AddonIndexAttachResult::from_domain_with_provider(
            attached,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn attach_collecting_progress(
        &self,
        request: AttachAddonIndexAppRequest,
    ) -> AppResult<TaskRun<AddonIndexAttachResult>> {
        task_support::run_service_task_collecting(self, request, Self::attach_task)
    }

    pub(super) fn attach_with_callbacks<FCancel, FProgress>(
        &self,
        request: AttachAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexAttachResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::attach_task,
        )
    }

    pub(super) fn install_task<TCancel, TProgress>(
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
            request.into_domain_request(&self.runtime)?,
            cancellation,
            progress,
        )?;
        Ok(AddonIndexInstallResult::from_domain_with_provider(
            installed,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn install_collecting_progress(
        &self,
        request: InstallAddonIndexAppRequest,
    ) -> AppResult<TaskRun<AddonIndexInstallResult>> {
        task_support::run_service_task_collecting(self, request, Self::install_task)
    }

    pub(super) fn install_with_callbacks<FCancel, FProgress>(
        &self,
        request: InstallAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexInstallResult>
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

    pub(super) fn update_task<TCancel, TProgress>(
        &self,
        request: UpdateAddonIndexAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonIndexUpdateResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let request = request.into_domain_request(&self.runtime)?;
        validate_addon_index_update_dependency_policy_support(
            self.runtime.addon_provider(),
            &request.installation,
            &request.state_paths,
            &request.index_path,
            request.name.as_deref(),
        )?;
        let updated = update_addons_from_index_task_with_provider(
            self.runtime.addon_provider(),
            request,
            cancellation,
            progress,
        )?;
        Ok(AddonIndexUpdateResult::from_domain_with_provider(
            updated,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn update_collecting_progress(
        &self,
        request: UpdateAddonIndexAppRequest,
    ) -> AppResult<TaskRun<AddonIndexUpdateResult>> {
        task_support::run_service_task_collecting(self, request, Self::update_task)
    }

    pub(super) fn update_with_callbacks<FCancel, FProgress>(
        &self,
        request: UpdateAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexUpdateResult>
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

    pub(super) fn relink_task<TCancel, TProgress>(
        &self,
        request: RelinkAddonIndexAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonIndexRelinkResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let relinked = relink_addon_from_index_task_with_provider(
            self.runtime.addon_provider(),
            request.into_domain_request(&self.runtime)?,
            cancellation,
            progress,
        )?;
        Ok(AddonIndexRelinkResult::from_domain_with_provider(
            relinked,
            self.runtime.addon_provider(),
        ))
    }

    pub(super) fn relink_collecting_progress(
        &self,
        request: RelinkAddonIndexAppRequest,
    ) -> AppResult<TaskRun<AddonIndexRelinkResult>> {
        task_support::run_service_task_collecting(self, request, Self::relink_task)
    }

    pub(super) fn relink_with_callbacks<FCancel, FProgress>(
        &self,
        request: RelinkAddonIndexAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexRelinkResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_service_task_with_callbacks(
            self,
            request,
            is_cancelled,
            on_progress,
            Self::relink_task,
        )
    }
}
#[cfg(test)]
mod tests;
