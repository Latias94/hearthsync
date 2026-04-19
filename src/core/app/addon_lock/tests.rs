use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use super::*;
use crate::core::app::ResolvedInstallationValue;
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase};

#[test]
fn addon_lock_service_plan_sync_reads_empty_lock() {
    let temp = tempdir().expect("temp dir");
    let current = create_empty_installation(&temp.path().join("current"));
    let lock_path = write_empty_lock(temp.path().join("desired-lock.toml"));

    let service = AddonLockService::new();
    let plan = service
        .plan_sync(PlanAddonLockSyncRequest {
            installation: current,
            lock_path: Some(lock_path),
        })
        .expect("plan addon lock");

    assert_eq!(plan.install_count, 0);
    assert_eq!(plan.update_count, 0);
    assert_eq!(plan.remove_count, 0);
}

#[test]
fn addon_lock_service_apply_sync_collecting_progress_returns_lock_task_events() {
    let temp = tempdir().expect("temp dir");
    let current = create_empty_installation(&temp.path().join("current"));
    let lock_path = write_empty_lock(temp.path().join("desired-lock.toml"));

    let service = AddonLockService::new();
    let run = service
        .apply_sync_collecting_progress(ApplyAddonLockAppRequest {
            installation: current,
            lock_path: Some(lock_path),
            backup_output_path: None,
            replace_existing: false,
            source_overrides: Vec::new(),
        })
        .expect("apply addon lock");

    assert!(run.result.verification.matches);
    assert_eq!(
        run.progress
            .iter()
            .map(|event| (event.task, event.phase))
            .collect::<Vec<_>>(),
        vec![
            (TaskKind::AddonLockApply, TaskPhase::Preparing),
            (TaskKind::AddonLockApply, TaskPhase::Planning),
            (TaskKind::AddonLockApply, TaskPhase::Verifying),
            (TaskKind::AddonLockApply, TaskPhase::Completed),
        ]
    );
}

#[test]
fn addon_lock_service_apply_sync_with_callbacks_uses_plain_closures() {
    let temp = tempdir().expect("temp dir");
    let current = create_empty_installation(&temp.path().join("current"));
    let lock_path = write_empty_lock(temp.path().join("desired-lock.toml"));

    let service = AddonLockService::new();
    let seen = RefCell::new(Vec::new());
    let cancellation_checks = Cell::new(0usize);
    let result = service
        .apply_sync_with_callbacks(
            ApplyAddonLockAppRequest {
                installation: current,
                lock_path: Some(lock_path),
                backup_output_path: None,
                replace_existing: false,
                source_overrides: Vec::new(),
            },
            || {
                let next = cancellation_checks.get() + 1;
                cancellation_checks.set(next);
                false
            },
            |event| seen.borrow_mut().push(event),
        )
        .expect("apply addon lock with callbacks");

    assert!(result.verification.matches);
    assert_eq!(seen.borrow().len(), 4);
    assert!(cancellation_checks.get() >= 3);
}

fn write_empty_lock(path: PathBuf) -> PathBuf {
    let lock = crate::core::addon::lock::AddonLock {
        schema_version: 1,
        generated_at: "2026-04-16T00:00:00Z".to_string(),
        packages: Vec::new(),
    };
    fs::write(&path, toml::to_string(&lock).expect("serialize lock")).expect("write empty lock");
    path
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
