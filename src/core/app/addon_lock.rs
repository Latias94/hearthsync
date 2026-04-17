use std::path::Path;

use crate::core::addon::lock::{
    AddonLockApplyRequest, AddonLockApplyResult, AddonLockDiffResult, AddonLockInspection,
    AddonLockPlanResult, AddonLockVerifyResult, AddonLockWriteResult, apply_addon_lock_sync,
    apply_addon_lock_sync_task, diff_addon_locks, inspect_addon_lock, plan_addon_lock_sync,
    verify_addon_lock, write_addon_lock,
};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, TaskProgressEvent, TaskProgressSink, TaskRun, run_task_with_callbacks,
    run_task_with_collected_progress,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct AddonLockService;

impl AddonLockService {
    pub fn new() -> Self {
        Self
    }

    pub fn inspect(
        &self,
        installation: &DetectedFlavorInstallation,
    ) -> AppResult<AddonLockInspection> {
        inspect_addon_lock(installation)
    }

    pub fn write(
        &self,
        installation: &DetectedFlavorInstallation,
    ) -> AppResult<AddonLockWriteResult> {
        write_addon_lock(installation)
    }

    pub fn diff(&self, left: &Path, right: &Path) -> AppResult<AddonLockDiffResult> {
        diff_addon_locks(left, right)
    }

    pub fn verify(
        &self,
        installation: &DetectedFlavorInstallation,
        lock_path: Option<&Path>,
    ) -> AppResult<AddonLockVerifyResult> {
        verify_addon_lock(installation, lock_path)
    }

    pub fn plan_sync(
        &self,
        installation: &DetectedFlavorInstallation,
        lock_path: Option<&Path>,
    ) -> AppResult<AddonLockPlanResult> {
        plan_addon_lock_sync(installation, lock_path)
    }

    pub fn apply_sync(&self, request: AddonLockApplyRequest) -> AppResult<AddonLockApplyResult> {
        apply_addon_lock_sync(request)
    }

    pub fn apply_sync_task<TCancel, TProgress>(
        &self,
        request: AddonLockApplyRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonLockApplyResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        apply_addon_lock_sync_task(request, cancellation, progress)
    }

    pub fn apply_sync_collecting_progress(
        &self,
        request: AddonLockApplyRequest,
    ) -> AppResult<TaskRun<AddonLockApplyResult>> {
        run_task_with_collected_progress(|cancellation, progress| {
            self.apply_sync_task(request, cancellation, progress)
        })
    }

    pub fn apply_sync_with_callbacks<FCancel, FProgress>(
        &self,
        request: AddonLockApplyRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonLockApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        run_task_with_callbacks(is_cancelled, on_progress, |cancellation, progress| {
            self.apply_sync_task(request, cancellation, progress)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::*;
    use crate::core::install::{HostPlatform, WowFlavor};
    use crate::core::task::{TaskKind, TaskPhase};

    #[test]
    fn addon_lock_service_plan_sync_reads_empty_lock() {
        let temp = tempdir().expect("temp dir");
        let current = create_empty_installation(&temp.path().join("current"));
        let lock_path = write_empty_lock(temp.path().join("desired-lock.toml"));

        let service = AddonLockService::new();
        let plan = service
            .plan_sync(&current, Some(&lock_path))
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
            .apply_sync_collecting_progress(AddonLockApplyRequest {
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
                AddonLockApplyRequest {
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
        fs::write(&path, toml::to_string(&lock).expect("serialize lock"))
            .expect("write empty lock");
        path
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
