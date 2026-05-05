use super::*;

#[test]
fn analyze_external_package_task_reports_progress() {
    let package_root = external_package_dirty_fixture_root();
    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;

    let analysis = analyze_external_package_task(
        AnalyzeExternalPackageRequest {
            source_path: package_root,
            layout: ExternalPackageLayout::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
        },
        &cancellation,
        &mut progress,
    )
    .expect("analyze external package task");

    assert_eq!(analysis.summary.warning_count, 1);
    assert_eq!(
        progress
            .events()
            .iter()
            .map(|event| (event.task, event.phase))
            .collect::<Vec<_>>(),
        vec![
            (TaskKind::ExternalPackageAnalyze, TaskPhase::Preparing),
            (TaskKind::ExternalPackageAnalyze, TaskPhase::Planning),
            (TaskKind::ExternalPackageAnalyze, TaskPhase::Completed),
        ]
    );
}

#[test]
fn plan_external_package_apply_task_reports_progress() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation = create_fixture_installation(target.path(), false);
    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let plan = plan_external_package_apply_task(
        PlanExternalPackageApplyRequest {
            external_package: CreateExternalPackageBundleRequest {
                source_path: package_path,
                layout: ExternalPackageLayout::Auto,
                source_account: None,
                source_server: None,
                source_character: None,
                source_flavor: WowFlavor::Retail,
                source_platform: Some(HostPlatform::Windows),
                supported_targets: vec![WowFlavor::Retail],
                output_path: None,
                package_id: None,
                package_name: None,
                created_by: None,
                description: None,
                apply_defaults: None,
                sharing_mode: ExternalPackageSharingMode::Private,
                allow_public_sharing_risks: false,
                excluded_wtf_scopes: Vec::new(),
            },
            installation: target_installation,
            apply_mappings: BundleApplyMappings {
                target_account: Some("ACCOUNT".to_string()),
                target_server: Some("Illidan".to_string()),
                target_character: Some("Examplemage".to_string()),
                ..BundleApplyMappings::default()
            },
        },
        &cancellation,
        &mut progress,
    )
    .expect("plan external package task");

    assert!(!plan.operations.is_empty());
    assert_eq!(
        progress
            .events()
            .iter()
            .map(|event| (event.task, event.phase))
            .collect::<Vec<_>>(),
        vec![
            (TaskKind::ExternalPackagePlan, TaskPhase::Preparing),
            (TaskKind::ExternalPackagePlan, TaskPhase::Planning),
            (TaskKind::ExternalPackagePlan, TaskPhase::Completed),
        ]
    );
}

#[test]
fn apply_external_package_task_wraps_normalization_and_apply_progress() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation = create_fixture_installation(target.path(), false);
    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = apply_external_package_task(
        ApplyExternalPackageRequest {
            external_package: CreateExternalPackageBundleRequest {
                source_path: package_path,
                layout: ExternalPackageLayout::Auto,
                source_account: None,
                source_server: None,
                source_character: None,
                source_flavor: WowFlavor::Retail,
                source_platform: Some(HostPlatform::Windows),
                supported_targets: vec![WowFlavor::Retail],
                output_path: None,
                package_id: None,
                package_name: None,
                created_by: None,
                description: None,
                apply_defaults: None,
                sharing_mode: ExternalPackageSharingMode::Private,
                allow_public_sharing_risks: false,
                excluded_wtf_scopes: Vec::new(),
            },
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings {
                target_account: Some("ACCOUNT".to_string()),
                target_server: Some("Illidan".to_string()),
                target_character: Some("Examplemage".to_string()),
                ..BundleApplyMappings::default()
            },
        },
        &cancellation,
        &mut progress,
    )
    .expect("apply external package task");

    assert!(result.written_files > 0);
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(target_installation.wtf_dir.join("Config.wtf").exists());
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.message.contains("Normalizing external package"))
    );
    assert!(
        progress
            .events()
            .iter()
            .all(|event| event.task == TaskKind::ExternalPackageApply)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Preparing)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Planning)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Executing)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Completed)
    );
    assert!(progress.events().iter().any(|event| {
        event.phase == TaskPhase::Executing && event.message.contains("operation 1/")
    }));

    let second_target = tempdir().expect("second target");
    let direct_result = apply_external_package(ApplyExternalPackageRequest {
        external_package: CreateExternalPackageBundleRequest {
            source_path: source.path().join("author-ui-pack.zip"),
            layout: ExternalPackageLayout::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
            source_flavor: WowFlavor::Retail,
            source_platform: Some(HostPlatform::Windows),
            supported_targets: vec![WowFlavor::Retail],
            output_path: None,
            package_id: Some("author-ui-direct".to_string()),
            package_name: Some("Author UI Direct".to_string()),
            created_by: None,
            description: None,
            apply_defaults: None,
            sharing_mode: ExternalPackageSharingMode::Private,
            allow_public_sharing_risks: false,
            excluded_wtf_scopes: Vec::new(),
        },
        installation: create_fixture_installation(second_target.path(), false),
        dry_run: true,
        backup_output_path: None,
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("apply external package directly");
    assert!(direct_result.dry_run);
}
