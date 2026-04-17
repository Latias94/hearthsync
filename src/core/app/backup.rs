use std::path::Path;

use crate::core::backup::{
    BackupCatalog, BackupRequest, CreatedBackup, RestoreBackupRequest, RestoredBackup,
    create_backup, list_backups, restore_backup_selection, restore_backup_selection_task,
};
use crate::core::error::AppResult;
use crate::core::task::{
    CancellationToken, TaskProgressEvent, TaskProgressSink, TaskRun, run_task_with_callbacks,
    run_task_with_collected_progress,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct BackupService;

impl BackupService {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, request: BackupRequest) -> AppResult<CreatedBackup> {
        create_backup(request)
    }

    pub fn list(&self, backup_dir: Option<&Path>) -> AppResult<BackupCatalog> {
        list_backups(backup_dir)
    }

    pub fn restore(&self, request: RestoreBackupRequest) -> AppResult<RestoredBackup> {
        restore_backup_selection(request)
    }

    pub fn restore_task<TCancel, TProgress>(
        &self,
        request: RestoreBackupRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<RestoredBackup>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        restore_backup_selection_task(request, cancellation, progress)
    }

    pub fn restore_collecting_progress(
        &self,
        request: RestoreBackupRequest,
    ) -> AppResult<TaskRun<RestoredBackup>> {
        run_task_with_collected_progress(|cancellation, progress| {
            self.restore_task(request, cancellation, progress)
        })
    }

    pub fn restore_with_callbacks<FCancel, FProgress>(
        &self,
        request: RestoreBackupRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<RestoredBackup>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        run_task_with_callbacks(is_cancelled, on_progress, |cancellation, progress| {
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
    use crate::core::backup::{BackupGroup, BackupRequest};
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
    use crate::core::task::{TaskKind, TaskPhase};

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
            .create(BackupRequest {
                installation: installation.clone(),
                output_path: Some(temp.path().join("backups")),
                groups: vec![BackupGroup::Addons],
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
            .restore_collecting_progress(RestoreBackupRequest {
                installation,
                archive_path: Some(backup.archive_path),
                backup_id: None,
                backup_dir: None,
            })
            .expect("restore with collected progress");

        assert_eq!(run.result.restored_files, 1);
        assert_eq!(
            run.progress
                .iter()
                .map(|event| (event.task, event.phase))
                .collect::<Vec<_>>(),
            vec![
                (TaskKind::BackupRestore, TaskPhase::Preparing),
                (TaskKind::BackupRestore, TaskPhase::BackingUp),
                (TaskKind::BackupRestore, TaskPhase::Executing),
                (TaskKind::BackupRestore, TaskPhase::Completed),
            ]
        );
    }

    #[test]
    fn backup_service_restore_with_callbacks_uses_plain_closures() {
        let temp = tempdir().expect("temp dir");
        let installation = create_empty_installation(temp.path());

        fs::create_dir_all(installation.wtf_dir.join("Account")).expect("wtf dir");
        fs::write(installation.wtf_dir.join("Config.wtf"), "before").expect("config");

        let service = BackupService::new();
        let backup = service
            .create(BackupRequest {
                installation: installation.clone(),
                output_path: Some(temp.path().join("backups")),
                groups: vec![BackupGroup::Wtf],
                label: Some("service-callback".to_string()),
            })
            .expect("create backup");

        fs::write(installation.wtf_dir.join("Config.wtf"), "after").expect("config");

        let seen = RefCell::new(Vec::new());
        let cancellation_checks = Cell::new(0usize);
        let restored = service
            .restore_with_callbacks(
                RestoreBackupRequest {
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
        assert_eq!(seen.borrow().len(), 4);
        assert!(cancellation_checks.get() >= 3);
    }

    fn create_empty_installation(root: &Path) -> DetectedFlavorInstallation {
        let product_root = root.join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        fs::create_dir_all(&addon_dir).expect("addon dir");
        fs::create_dir_all(&wtf_dir).expect("wtf dir");
        fs::create_dir_all(&fonts_dir).expect("fonts dir");

        DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root,
            flavor_root,
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        }
    }
}
