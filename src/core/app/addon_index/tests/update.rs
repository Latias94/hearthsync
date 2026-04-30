use super::*;

#[test]
fn addon_index_service_update_with_callbacks_uses_plain_closures() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let installed_archive_path = temp.path().join("Details-installed.zip");
    let updated_archive_path = temp.path().join("Details-updated.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);
    let domain_installation = installation
        .clone()
        .into_domain()
        .expect("resolved installation");

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&domain_installation),
        installation: domain_installation,
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let seen = std::cell::RefCell::new(Vec::new());
    let cancellation_checks = std::cell::Cell::new(0usize);
    let result = service_update_with_callbacks(
        &AddonIndexService::new(),
        UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("details".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &seen,
        &cancellation_checks,
    )
    .expect("update from index with callbacks");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(seen.borrow().len() >= 4);
    assert!(seen.borrow().iter().any(|event| {
        event.task == TaskKind::AddonIndexUpdate
            && event.phase == TaskPhase::Executing
            && event.message.contains("Writing updated addon directory")
    }));
    assert!(cancellation_checks.get() >= 3);
}

fn service_update_with_callbacks(
    service: &AddonIndexService,
    request: UpdateAddonIndexAppRequest,
    seen: &std::cell::RefCell<Vec<TaskProgressEvent>>,
    cancellation_checks: &std::cell::Cell<usize>,
) -> AppResult<AddonIndexUpdateResult> {
    service.update_with_callbacks(
        request,
        || {
            let next = cancellation_checks.get() + 1;
            cancellation_checks.set(next);
            false
        },
        |event| seen.borrow_mut().push(event),
    )
}

#[test]
fn addon_index_service_update_preflights_unsupported_dependency_policy_before_domain_prepare() {
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
    let index_path = temp.path().join("addon-index-github.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = { kind = "github_release", owner = "owner", repo = "repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeUnsupportedDependencyAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
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

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("unsupported dependency policy should fail in app preflight");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 0);
}

#[test]
fn addon_index_service_update_preflights_unsupported_dependency_policy_by_display_name_when_source_family_changes()
 {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("addon-index-github-display-name.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Plater"
version = "2.0.0"
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeDisplayNamePreflightAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
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

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("unsupported dependency policy should fail in app preflight");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 0);
}

#[test]
fn addon_index_service_update_preflights_unsupported_dependency_policy_by_curated_package_hint_when_source_family_changes()
 {
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
    let index_path = temp.path().join("addon-index-github-hint.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater-v2"
name = "Curated Plater Package"
version = "2.0.0"
match_package_ids = ["plater"]
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeDisplayNamePreflightAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
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

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater-v2".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("unsupported dependency policy should fail in app preflight");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 0);
}

#[test]
fn addon_index_service_update_reports_deferred_dependency_policy_guidance_when_preflight_cannot_match()
 {
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
    let index_path = temp.path().join("addon-index-github-domain-fallback.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater-v3"
name = "Curated Plater Package"
version = "2.0.0"
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let update_attempts = Arc::new(AtomicUsize::new(0));
    let runtime = AppRuntime::with_addon_provider(FakeDeferredDependencyGuidanceAddonProvider {
        archive_path: archive_path.clone(),
        update_attempts: update_attempts.clone(),
    });
    let addon_service = AddonService::with_runtime(runtime.clone());
    let index_service = AddonIndexService::with_runtime(runtime);

    addon_service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
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

    let error = index_service
        .update(UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-plater-v3".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        })
        .expect_err("domain fallback should fail with explicit guidance");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert!(
        error
            .to_string()
            .contains("app preflight could not determine")
    );
    assert!(error.to_string().contains("match_package_ids"));
    assert_eq!(update_attempts.load(Ordering::SeqCst), 1);
}
