use super::*;

#[test]
fn unpack_bundle_task_reports_progress_for_dry_run() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation,
            dry_run: true,
            backup_output_path: None,
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect("unpack bundle dry run task");

    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases,
        vec![
            (TaskKind::BundleApply, TaskPhase::Preparing),
            (TaskKind::BundleApply, TaskPhase::Planning),
            (TaskKind::BundleApply, TaskPhase::Completed),
        ]
    );
    assert!(result.dry_run);
}

#[test]
fn unpack_bundle_task_honors_cancellation_before_execution() {
    struct CancelOnSecondCheck {
        checks: Cell<usize>,
    }

    impl CancellationToken for CancelOnSecondCheck {
        fn is_cancelled(&self) -> bool {
            let next = self.checks.get() + 1;
            self.checks.set(next);
            next >= 2
        }
    }

    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = CancelOnSecondCheck {
        checks: Cell::new(0),
    };
    let error = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("bundle task should cancel before execution");

    assert!(matches!(error, crate::core::error::AppError::Cancelled(_)));
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn unpack_bundle_task_reports_operation_progress_during_execution() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation,
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect("bundle task should complete");

    let executing_messages = progress
        .events()
        .iter()
        .filter(|event| event.task == TaskKind::BundleApply && event.phase == TaskPhase::Executing)
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();

    assert!(result.written_files > 0);
    assert!(executing_messages.len() > 1);
    assert!(executing_messages.iter().any(|message| {
        message.contains("operation 1/") && message.contains("Executing bundle operation")
    }));
}

#[test]
fn unpack_bundle_task_honors_cancellation_during_execution_loop() {
    struct CancelOnFifthCheck {
        checks: Cell<usize>,
    }

    impl CancellationToken for CancelOnFifthCheck {
        fn is_cancelled(&self) -> bool {
            let next = self.checks.get() + 1;
            self.checks.set(next);
            next >= 5
        }
    }

    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = CancelOnFifthCheck {
        checks: Cell::new(0),
    };
    let error = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("bundle task should cancel during execution loop");

    assert!(matches!(error, crate::core::error::AppError::Cancelled(_)));
    assert!(progress.events().iter().any(|event| {
        event.task == TaskKind::BundleApply
            && event.phase == TaskPhase::Executing
            && event.message.contains("operation 1/")
    }));
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}
