use super::*;

#[test]
fn apply_addon_lock_sync_updates_installs_and_removes_packages() {
    let temp = tempdir().expect("temp dir");
    let source_root = temp.path().join("sources");
    fs::create_dir_all(&source_root).expect("source root");

    let details_v1 = source_root.join("details-v1.zip");
    let details_v2 = source_root.join("details-v2.zip");
    let omen = source_root.join("omen.zip");
    let bigwigs = source_root.join("bigwigs.zip");
    create_addon_archive(
        &details_v1,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &details_v2,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 2.0.0\n",
        )],
    );
    create_addon_archive(
        &omen,
        &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
    );
    create_addon_archive(
        &bigwigs,
        &[(
            "BigWigs/BigWigs.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let desired_installation = create_fixture_installation(&temp.path().join("desired"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&desired_installation.clone()),
        installation: desired_installation.clone(),
        source: details_v2.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install desired details");
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&desired_installation.clone()),
        installation: desired_installation.clone(),
        source: bigwigs.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install desired bigwigs");
    let desired_lock = write_addon_lock(
        &desired_installation,
        &addon_state_paths(&desired_installation),
    )
    .expect("write desired lock")
    .lock_path;

    let current_installation = create_fixture_installation(&temp.path().join("current"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&current_installation.clone()),
        installation: current_installation.clone(),
        source: details_v1.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current details");
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&current_installation.clone()),
        installation: current_installation.clone(),
        source: omen.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current omen");

    let plan = plan_addon_lock_sync(
        &current_installation,
        &addon_state_paths(&current_installation),
        Some(&desired_lock),
    )
    .expect("plan");
    assert_eq!(plan.install_count, 1);
    assert_eq!(plan.update_count, 1);
    assert_eq!(plan.remove_count, 1);
    assert_eq!(plan.blocked_count, 0);

    let apply_backup_dir = temp.path().join("apply-backups");
    let result = apply_addon_lock_sync(AddonLockApplyRequest {
        state_paths: addon_state_paths(&current_installation.clone()),
        installation: current_installation.clone(),
        lock_path: Some(desired_lock.clone()),
        backup_output_path: Some(apply_backup_dir.clone()),
        replace_existing: false,
        source_overrides: Vec::new(),
    })
    .expect("apply lock sync");

    assert!(result.verification.matches);
    assert_eq!(result.install_count, 1);
    assert_eq!(result.update_count, 1);
    assert_eq!(result.remove_count, 1);
    assert!(
        fs::read_to_string(
            current_installation
                .addon_dir
                .join("Details")
                .join("Details.toc")
        )
        .expect("details toc")
        .contains("2.0.0")
    );
    assert!(current_installation.addon_dir.join("BigWigs").exists());
    assert!(!current_installation.addon_dir.join("Omen").exists());
    assert_eq!(count_backup_archives(&apply_backup_dir), 1);
}

#[test]
fn apply_addon_lock_sync_task_reports_progress() {
    let temp = tempdir().expect("temp dir");
    let source_root = temp.path().join("sources");
    fs::create_dir_all(&source_root).expect("source root");

    let details_v1 = source_root.join("details-v1.zip");
    let details_v2 = source_root.join("details-v2.zip");
    create_addon_archive(
        &details_v1,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &details_v2,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 2.0.0\n",
        )],
    );

    let desired_installation = create_fixture_installation(&temp.path().join("desired"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&desired_installation.clone()),
        installation: desired_installation.clone(),
        source: details_v2.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install desired details");
    let desired_lock = write_addon_lock(
        &desired_installation,
        &addon_state_paths(&desired_installation),
    )
    .expect("write desired lock")
    .lock_path;

    let current_installation = create_fixture_installation(&temp.path().join("current"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&current_installation.clone()),
        installation: current_installation.clone(),
        source: details_v1.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current details");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = apply_addon_lock_sync_task(
        AddonLockApplyRequest {
            state_paths: addon_state_paths(&current_installation),
            installation: current_installation,
            lock_path: Some(desired_lock),
            backup_output_path: Some(temp.path().join("apply-backups")),
            replace_existing: false,
            source_overrides: Vec::new(),
        },
        &cancellation,
        &mut progress,
    )
    .expect("apply addon lock task");

    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases.first(),
        Some(&(TaskKind::AddonLockApply, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::AddonLockApply, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::AddonLockApply, TaskPhase::Planning)));
    assert!(phases.contains(&(TaskKind::AddonLockApply, TaskPhase::BackingUp)));
    assert!(phases.contains(&(TaskKind::AddonLockApply, TaskPhase::Verifying)));
    assert!(
        phases
            .iter()
            .any(|phase| { *phase == (TaskKind::AddonLockApply, TaskPhase::Executing) })
    );
    assert!(progress.events().iter().any(|event| {
        event.task == TaskKind::AddonLockApply
            && event.phase == TaskPhase::Executing
            && (event.message.contains("Removing")
                || event.message.contains("Writing updated addon directory")
                || event.message.contains("Installing addon directory"))
    }));
    assert!(result.verification.matches);
}

#[test]
fn apply_addon_lock_sync_task_rolls_back_when_verification_is_cancelled() {
    let temp = tempdir().expect("temp dir");
    let source_root = temp.path().join("sources");
    fs::create_dir_all(&source_root).expect("source root");

    let details_v1 = source_root.join("details-v1.zip");
    let details_v2 = source_root.join("details-v2.zip");
    create_addon_archive(
        &details_v1,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &details_v2,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 2.0.0\n",
        )],
    );

    let desired_installation = create_fixture_installation(&temp.path().join("desired"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&desired_installation.clone()),
        installation: desired_installation.clone(),
        source: details_v2.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install desired details");
    let desired_lock = write_addon_lock(
        &desired_installation,
        &addon_state_paths(&desired_installation),
    )
    .expect("write desired lock")
    .lock_path;

    let current_installation = create_fixture_installation(&temp.path().join("current"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&current_installation.clone()),
        installation: current_installation.clone(),
        source: details_v1.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current details");

    let apply_backup_dir = temp.path().join("apply-backups");
    let cancellation = CancelDuringVerifying::default();
    let mut progress = CancelDuringVerifyingProgressSink::new(&cancellation.cancel_requested);
    let error = apply_addon_lock_sync_task(
        AddonLockApplyRequest {
            state_paths: addon_state_paths(&current_installation.clone()),
            installation: current_installation.clone(),
            lock_path: Some(desired_lock),
            backup_output_path: Some(apply_backup_dir.clone()),
            replace_existing: false,
            source_overrides: Vec::new(),
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("verification cancellation should roll back addon lock apply");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("rollback restored"));
    assert!(error.to_string().contains("cancelled during verifying"));
    assert!(
        fs::read_to_string(
            current_installation
                .addon_dir
                .join("Details")
                .join("Details.toc")
        )
        .expect("details toc after rollback")
        .contains("1.0.0")
    );
    assert_eq!(count_backup_archives(&apply_backup_dir), 1);

    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert!(phases.contains(&(TaskKind::AddonLockApply, TaskPhase::Verifying)));
    assert!(!phases.contains(&(TaskKind::AddonLockApply, TaskPhase::Completed)));
}

#[test]
fn apply_addon_lock_sync_applies_metadata_only_actions_transactionally() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("details-pack.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let desired_installation = create_fixture_installation(&temp.path().join("desired"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&desired_installation.clone()),
        installation: desired_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("desired-backups")),
        replace_existing: false,
        metadata: Some(AddonPackageMetadata {
            package_name: Some("Curated Details".to_string()),
            version: Some("1.0.0-curated".to_string()),
            source_url: Some("https://example.invalid/details".to_string()),
            website_url: Some("https://example.invalid/details/site".to_string()),
            ..AddonPackageMetadata::default()
        }),
    })
    .expect("install desired details");
    let desired_lock = write_addon_lock(
        &desired_installation,
        &addon_state_paths(&desired_installation),
    )
    .expect("write desired lock")
    .lock_path;

    let current_installation = create_fixture_installation(&temp.path().join("current"));
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&current_installation.clone()),
        installation: current_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("current-backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install current details");

    let plan = plan_addon_lock_sync(
        &current_installation,
        &addon_state_paths(&current_installation),
        Some(&desired_lock),
    )
    .expect("plan");
    assert_eq!(plan.install_count, 0);
    assert_eq!(plan.update_count, 0);
    assert_eq!(plan.remove_count, 0);
    assert_eq!(plan.metadata_only_count, 1);

    let apply_backup_dir = temp.path().join("metadata-apply-backups");
    let result = apply_addon_lock_sync(AddonLockApplyRequest {
        state_paths: addon_state_paths(&current_installation.clone()),
        installation: current_installation.clone(),
        lock_path: Some(desired_lock),
        backup_output_path: Some(apply_backup_dir.clone()),
        replace_existing: false,
        source_overrides: Vec::new(),
    })
    .expect("apply metadata-only lock sync");

    assert!(result.verification.matches);
    assert_eq!(result.metadata_only_count, 1);
    assert_eq!(count_backup_archives(&apply_backup_dir), 1);

    let inventory = list_addons(
        &current_installation,
        &addon_state_paths(&current_installation),
    )
    .expect("list addons");
    let metadata = inventory.tracked_packages[0]
        .metadata
        .as_ref()
        .expect("metadata");
    assert_eq!(metadata.package_name.as_deref(), Some("Curated Details"));
    assert_eq!(metadata.version.as_deref(), Some("1.0.0-curated"));
    assert_eq!(
        metadata.source_url.as_deref(),
        Some("https://example.invalid/details")
    );
}
