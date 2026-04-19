use crate::core::app::ListBackupsRequest;
use crate::core::app::{
    AppRuntime, BackupCatalogResult, CancellationToken, CreateBackupAppRequest,
    CreatedBackupResult, RestoreBackupAppRequest, RestoredBackupResult, TaskProgressEvent,
    TaskProgressSink, TaskRun, task_support,
};
use crate::core::backup::{create_backup, list_backups, restore_backup_selection_task};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct BackupService {
    runtime: AppRuntime,
}

impl BackupService {
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

    pub fn create(&self, request: CreateBackupAppRequest) -> AppResult<CreatedBackupResult> {
        let created = create_backup(request.into_domain_request(&self.runtime))?;
        Ok(CreatedBackupResult::from_domain(created))
    }

    pub fn list(&self, request: ListBackupsRequest) -> AppResult<BackupCatalogResult> {
        let backup_dir = request.into_backup_dir(&self.runtime);
        let catalog = list_backups(backup_dir.as_deref())?;
        Ok(BackupCatalogResult::from_domain(catalog))
    }

    pub fn restore(&self, request: RestoreBackupAppRequest) -> AppResult<RestoredBackupResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.restore_task(request, cancellation, progress)
        })
    }

    pub fn restore_task<TCancel, TProgress>(
        &self,
        request: RestoreBackupAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<RestoredBackupResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let restored = restore_backup_selection_task(
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(RestoredBackupResult::from_domain(restored))
    }

    pub fn restore_collecting_progress(
        &self,
        request: RestoreBackupAppRequest,
    ) -> AppResult<TaskRun<RestoredBackupResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.restore_task(request, cancellation, progress)
        })
    }

    pub fn restore_with_callbacks<FCancel, FProgress>(
        &self,
        request: RestoreBackupAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<RestoredBackupResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.restore_task(request, cancellation, progress)
        })
    }
}
#[cfg(test)]
mod tests;
