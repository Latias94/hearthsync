use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::core::app::{
    AppRuntime, BackupService, CreateBackupAppRequest, ListBackupsRequest,
    ResolvedInstallationValue, RestoreBackupAppRequest,
};
use crate::core::error::AppError;
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent};

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
    assert!(run.task_id.starts_with("task-"));
    assert!(
        run.progress
            .iter()
            .all(|event| event.task_id.as_deref() == Some(run.task_id.as_str()))
    );
    let restore_entry = run
        .progress
        .iter()
        .find(|event| event.code == Some(TaskProgressCode::RestoreEntry))
        .expect("restore entry progress");
    assert_eq!(restore_entry.current, Some(1));
    assert_eq!(restore_entry.total, Some(1));
    assert_backup_restore_task_progress(&run.progress);
}

#[test]
fn backup_service_list_resolves_relative_dir_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let backup_dir = temp.path().join("backups");
    fs::create_dir_all(&backup_dir).expect("backup dir");

    let service = BackupService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(temp.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    let catalog = service
        .list(ListBackupsRequest {
            backup_dir: Some(PathBuf::from("backups")),
        })
        .expect("list relative backup dir");

    assert_eq!(catalog.backup_dir, backup_dir);
}

#[test]
fn backup_service_create_resolves_relative_output_against_runtime_base() {
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

    let service = BackupService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(temp.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    let created = service
        .create(CreateBackupAppRequest {
            installation,
            output_path: Some(PathBuf::from("backups")),
            groups: vec![crate::core::app::BackupGroupValue::Addons],
            label: Some("relative-output".to_string()),
        })
        .expect("create relative backup output");

    let expected_output = temp.path().join("backups");
    assert_eq!(
        created.archive_path.parent(),
        Some(expected_output.as_path())
    );
}

#[test]
fn backup_service_create_rejects_relative_output_without_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());

    let error = BackupService::new()
        .create(CreateBackupAppRequest {
            installation,
            output_path: Some(PathBuf::from("backups")),
            groups: vec![crate::core::app::BackupGroupValue::Addons],
            label: None,
        })
        .expect_err("relative backup output without base should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("relative path base"));
}

#[test]
fn backup_service_list_rejects_relative_dir_without_runtime_base() {
    let error = BackupService::new()
        .list(ListBackupsRequest {
            backup_dir: Some(PathBuf::from("backups")),
        })
        .expect_err("relative backup dir without base should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("relative path base"));
}

#[test]
fn backup_service_restore_resolves_relative_archive_against_runtime_base() {
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

    let backup = BackupService::new()
        .create(CreateBackupAppRequest {
            installation: installation.clone(),
            output_path: Some(temp.path().join("backups")),
            groups: vec![crate::core::app::BackupGroupValue::Addons],
            label: Some("relative-restore".to_string()),
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

    let backup_file_name =
        PathBuf::from(backup.archive_path.file_name().expect("backup file name"));
    let service = BackupService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(
                backup
                    .archive_path
                    .parent()
                    .expect("backup parent")
                    .to_path_buf(),
            ))
            .build()
            .expect("runtime"),
    );
    let restored = service
        .restore(RestoreBackupAppRequest {
            installation,
            archive_path: Some(backup_file_name),
            backup_id: None,
            backup_dir: None,
        })
        .expect("restore relative backup archive");

    assert_eq!(restored.restored_files, 1);
}

#[test]
fn backup_service_restore_rejects_relative_archive_without_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());

    let error = BackupService::new()
        .restore(RestoreBackupAppRequest {
            installation,
            archive_path: Some(PathBuf::from("backup.zip")),
            backup_id: None,
            backup_dir: None,
        })
        .expect_err("relative backup archive without base should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("relative path base"));
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
        AppRuntime::builder()
            .with_default_backup_dir(Some(backup_dir.clone()))
            .build()
            .expect("runtime"),
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

    ResolvedInstallationValue::from_domain(crate::core::install::DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    })
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
