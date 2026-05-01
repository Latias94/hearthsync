use super::*;

#[test]
fn addon_service_install_collecting_progress_returns_install_task_events() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    let run = service
        .install_collecting_progress(InstallAddonAppRequest {
            installation,
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon with collected progress");

    assert_eq!(run.result.package_id, "weakauras");
    assert!(run.task_id.starts_with("task-"));
    assert!(
        run.progress
            .iter()
            .all(|event| event.task_id.as_deref() == Some(run.task_id.as_str()))
    );
    let step = run
        .progress
        .iter()
        .find(|event| {
            event.task == TaskKind::AddonInstall
                && event.phase == TaskPhase::Executing
                && event.message.contains("Installing addon directory")
        })
        .expect("install step event");
    assert_eq!(step.code, Some(TaskProgressCode::WriteAddonDirectory));
    assert_eq!(step.current, Some(1));
    assert_eq!(step.total, Some(1));
    assert_addon_task_progress(
        &run.progress,
        TaskKind::AddonInstall,
        "Installing addon directory",
    );
}

#[test]
fn addon_service_install_with_runtime_uses_injected_provider() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras-runtime.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::with_runtime(AppRuntime::with_addon_provider(FakeAddonProvider {
        archive_path: archive_path.clone(),
    }));
    let installed = service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "https://example.invalid/WeakAuras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon through injected provider");
    let inventory = service
        .list(ListAddonsRequest { installation })
        .expect("list addons");

    assert_eq!(installed.package_id, "weakauras");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(
        inventory.tracked_packages[0].source.kind,
        crate::core::app::AddonSourceKindResult::HttpArchive
    );
    assert_eq!(
        inventory.tracked_packages[0].source.url.as_deref(),
        Some("https://example.invalid/WeakAuras.zip")
    );
    assert_eq!(
        inventory.tracked_packages[0]
            .source
            .dependency_resolution_capability,
        AddonDependencyResolutionCapabilityValue::Unsupported
    );
}

#[test]
fn addon_service_install_collecting_progress_includes_download_byte_events() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras-progress.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::with_runtime(AppRuntime::with_addon_provider(
        FakeDownloadProgressAddonProvider {
            archive_path: archive_path.clone(),
        },
    ));
    let run = service
        .install_collecting_progress(InstallAddonAppRequest {
            installation,
            source: "https://example.invalid/WeakAuras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon with byte progress");

    let download_events = run
        .progress
        .iter()
        .filter(|event| event.code == Some(TaskProgressCode::DownloadArchive))
        .collect::<Vec<_>>();
    assert_eq!(download_events.len(), 2);
    assert!(
        download_events
            .iter()
            .all(|event| event.phase == TaskPhase::Preparing)
    );
    assert_eq!(download_events[0].bytes_current, Some(0));
    assert_eq!(download_events[0].bytes_total, Some(1024));
    assert_eq!(download_events[1].bytes_current, Some(1024));
    assert_eq!(download_events[1].bytes_total, Some(1024));
    assert_eq!(download_events[1].bytes_per_second, Some(512));
}

#[test]
fn addon_service_update_with_callbacks_uses_plain_closures() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");

    create_addon_archive(
        &archive_path,
        &[
            (
                "Details/Details.toc",
                "## Interface: 120000\n## Version: 2.0.0\n",
            ),
            ("Details/Core.lua", "print('updated')"),
        ],
    );

    let seen = RefCell::new(Vec::new());
    let cancellation_checks = Cell::new(0usize);
    let result = service
        .update_with_callbacks(
            UpdateAddonAppRequest {
                installation,
                name: Some("Details".to_string()),
                dry_run: false,
                backup_output_path: Some(temp.path().join("backups")),
            },
            || {
                let next = cancellation_checks.get() + 1;
                cancellation_checks.set(next);
                false
            },
            |event| seen.borrow_mut().push(event),
        )
        .expect("update addon with callbacks");

    assert_eq!(result.updated_packages.len(), 1);
    assert!(seen.borrow().len() >= 4);
    let callback_task_id = seen.borrow()[0].task_id.clone();
    assert!(
        callback_task_id
            .as_deref()
            .is_some_and(|task_id| task_id.starts_with("task-"))
    );
    assert!(
        seen.borrow()
            .iter()
            .all(|event| event.task_id == callback_task_id)
    );
    assert!(seen.borrow().iter().any(|event| {
        event.task == TaskKind::AddonUpdate
            && event.phase == TaskPhase::Executing
            && event.message.contains("Writing updated addon directory")
            && event.code == Some(TaskProgressCode::WriteAddonDirectory)
            && event.current == Some(1)
            && event.total == Some(1)
    }));
    assert!(cancellation_checks.get() >= 3);
}

#[test]
fn addon_service_update_preflights_unsupported_dependency_policy_before_domain_prepare() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeUnsupportedDependencyAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let service = AddonService::with_runtime(runtime);

    service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked github addon");

    AddonPolicyService::new()
        .set(SetAddonPolicyAppRequest {
            installation: installation.clone(),
            package: "plater".to_string(),
            ignored: None,
            pin: None,
            release_channel: None,
            allow_prerelease: None,
            install_dependencies: Some(true),
        })
        .expect("enable dependency installation");

    let error = service
        .update(UpdateAddonAppRequest {
            installation,
            name: Some("plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("unsupported dependency policy should fail in app preflight");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 0);
}

#[test]
fn addon_service_remove_task_returns_remove_task_events() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = service
        .remove_task(
            RemoveAddonAppRequest {
                installation,
                name: "Plater".to_string(),
                dry_run: false,
                backup_output_path: Some(temp.path().join("backups")),
            },
            &cancellation,
            &mut progress,
        )
        .expect("remove addon task");

    assert_eq!(result.removed_addons, vec!["Plater".to_string()]);
    assert_addon_task_progress(
        progress.events(),
        TaskKind::AddonRemove,
        "Removing addon directory",
    );
}
