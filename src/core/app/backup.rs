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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn create(&self, request: CreateBackupAppRequest) -> AppResult<CreatedBackupResult> {
        let created = create_backup(request.apply_runtime_defaults(&self.runtime).into())?;
        Ok(CreatedBackupResult::from_domain(created))
    }

    pub fn list(&self, request: ListBackupsRequest) -> AppResult<BackupCatalogResult> {
        let request = request.apply_runtime_defaults(&self.runtime);
        let catalog = list_backups(request.backup_dir.as_deref())?;
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
            request.apply_runtime_defaults(&self.runtime).into(),
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
mod tests {
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::core::app::{AppRuntime, ResolvedInstallationValue};
    use crate::core::install::{HostPlatform, WowFlavor};
    use crate::core::task::{TaskKind, TaskPhase, TaskProgressEvent};

    #[test]
    fn backup_service_restore_collecting_progress_returns_restore_task_events() {
        let temp = tempdir().expect("temp dir");
        let installation = create_empty_installation(temp.path());

        fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
        fs::write(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc"),
            "before",
        )
        .expect("write toc");

        let service = BackupService::new();
        let backup = service
            .create(CreateBackupAppRequest {
                installation: installation.clone(),
                output_path: Some(temp.path().join("backups")),
                groups: vec![crate::core::app::BackupGroupValue::Addons],
                label: Some("service-restore".to_string()),
            })
            .expect("create backup");

        fs::write(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc"),
            "after",
        )
        .expect("mutate addon");

        let run = service
            .restore_collecting_progress(RestoreBackupAppRequest {
                installation,
                archive_path: Some(backup.archive_path),
                backup_id: None,
                backup_dir: None,
            })
            .expect("restore with collected progress");

        assert_eq!(run.result.restored_files, 1);
        assert_backup_restore_task_progress(&run.progress);
    }

    #[test]
    fn backup_service_restore_with_callbacks_uses_plain_closures() {
        let temp = tempdir().expect("temp dir");
        let installation = create_empty_installation(temp.path());

        fs::create_dir_all(installation.wtf_dir.join("Account")).expect("wtf dir");
        fs::write(installation.wtf_dir.join("Config.wtf"), "before").expect("config");

        let service = BackupService::new();
        let backup = service
            .create(CreateBackupAppRequest {
                installation: installation.clone(),
                output_path: Some(temp.path().join("backups")),
                groups: vec![crate::core::app::BackupGroupValue::Wtf],
                label: Some("service-callback".to_string()),
            })
            .expect("create backup");

        fs::write(installation.wtf_dir.join("Config.wtf"), "after").expect("config");

        let seen = RefCell::new(Vec::new());
        let cancellation_checks = Cell::new(0usize);
        let restored = service
            .restore_with_callbacks(
                RestoreBackupAppRequest {
                    installation,
                    archive_path: Some(backup.archive_path),
                    backup_id: None,
                    backup_dir: None,
                },
                || {
                    let next = cancellation_checks.get() + 1;
                    cancellation_checks.set(next);
                    false
                },
                |event| seen.borrow_mut().push(event),
            )
            .expect("restore with callbacks");

        assert_eq!(restored.restored_files, 1);
        assert!(seen.borrow().len() >= 4);
        assert!(seen.borrow().iter().any(|event| {
            event.task == TaskKind::BackupRestore
                && event.phase == TaskPhase::Executing
                && (event.message.contains("Clearing restore target group")
                    || event.message.contains("Restoring backup entry"))
        }));
        assert!(cancellation_checks.get() >= 3);
    }

    #[test]
    fn backup_service_uses_runtime_default_backup_dir_for_create_list_and_restore() {
        let temp = tempdir().expect("temp dir");
        let installation = create_empty_installation(temp.path());
        let backup_dir = temp.path().join("runtime-backups");

        fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
        fs::write(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc"),
            "before",
        )
        .expect("write toc");

        let service = BackupService::with_runtime(
            AppRuntime::new().with_default_backup_dir(Some(backup_dir.clone())),
        );
        let created = service
            .create(CreateBackupAppRequest {
                installation: installation.clone(),
                output_path: None,
                groups: vec![crate::core::app::BackupGroupValue::Addons],
                label: Some("runtime-default".to_string()),
            })
            .expect("create backup");
        let catalog = service
            .list(ListBackupsRequest { backup_dir: None })
            .expect("list backups");

        fs::write(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc"),
            "after",
        )
        .expect("mutate toc");

        let backup_id = created
            .archive_path
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("backup id")
            .to_string();
        let restored = service
            .restore(RestoreBackupAppRequest {
                installation,
                archive_path: None,
                backup_id: Some(backup_id),
                backup_dir: None,
            })
            .expect("restore backup with runtime default dir");

        assert_eq!(created.archive_path.parent(), Some(backup_dir.as_path()));
        assert_eq!(catalog.backup_dir, backup_dir);
        assert_eq!(catalog.entry_count, 1);
        assert_eq!(restored.restored_files, 1);
    }

    fn create_empty_installation(root: &Path) -> ResolvedInstallationValue {
        let product_root = root.join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        fs::create_dir_all(&addon_dir).expect("addon dir");
        fs::create_dir_all(&wtf_dir).expect("wtf dir");
        fs::create_dir_all(&fonts_dir).expect("fonts dir");

        crate::core::install::DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root,
            flavor_root,
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        }
        .into()
    }

    fn assert_backup_restore_task_progress(events: &[TaskProgressEvent]) {
        let phases = events
            .iter()
            .map(|event| (event.task, event.phase))
            .collect::<Vec<_>>();

        assert_eq!(
            phases.first(),
            Some(&(TaskKind::BackupRestore, TaskPhase::Preparing))
        );
        assert_eq!(
            phases.last(),
            Some(&(TaskKind::BackupRestore, TaskPhase::Completed))
        );
        assert!(phases.contains(&(TaskKind::BackupRestore, TaskPhase::BackingUp)));
        assert!(
            phases
                .iter()
                .any(|phase| { *phase == (TaskKind::BackupRestore, TaskPhase::Executing) })
        );
        assert!(events.iter().any(|event| {
            event.task == TaskKind::BackupRestore
                && event.phase == TaskPhase::Executing
                && (event.message.contains("Clearing restore target group")
                    || event.message.contains("Restoring backup entry"))
        }));
    }
}
