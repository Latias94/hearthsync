use super::*;

#[test]
fn addon_index_service_install_collecting_progress_returns_index_task_events() {
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
    let index_path = write_index(temp.path(), &archive_path);

    let service = AddonIndexService::new();
    let run = service
        .install_collecting_progress(InstallAddonIndexAppRequest {
            installation,
            index_path,
            name: "weakauras".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        })
        .expect("install from index with collected progress");

    assert_eq!(run.result.package.id, "weakauras");
    assert_addon_index_task_progress(
        &run.progress,
        TaskKind::AddonIndexInstall,
        "Installing addon directory",
    );
}

#[test]
fn addon_index_service_relink_attaches_curated_metadata_without_reinstalling_files() {
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
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
source_url = "https://example.invalid/details.zip"
supported_flavors = ["retail"]
addon_directories = ["Details"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install details");

    let service = AddonIndexService::new();
    let relinked = service
        .relink(RelinkAddonIndexAppRequest {
            installation: installation.clone(),
            index_path,
            name: "curated-details".to_string(),
            target: Some("details".to_string()),
            dry_run: false,
        })
        .expect("relink addon index");
    let inventory = AddonService::new()
        .list(crate::core::app::ListAddonsRequest {
            installation: installation.clone(),
        })
        .expect("list addons");

    assert_eq!(relinked.tracked_package_id, "details");
    assert!(!relinked.source_changed);
    assert!(relinked.metadata_changed);
    assert_eq!(
        relinked.metadata.index_package_id.as_deref(),
        Some("curated-details")
    );
    assert_eq!(
        inventory.tracked_packages[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.index_package_id.as_deref()),
        Some("curated-details")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("1.0.0")
    );
}

#[test]
fn addon_index_service_attach_blocks_without_partial_registry_writes() {
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

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");

    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "unknown-addon"
name = "Unknown Addon"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/unknown.zip" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .attach(AttachAddonIndexAppRequest {
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            apply_ready_only: false,
        })
        .expect("attach addon index");

    assert!(!result.ready);
    assert!(!result.applied);
    assert_eq!(result.blocked_package_count, 1);
    assert_eq!(result.change_package_count, 1);
    assert!(matches!(
        result.packages[0].status,
        AddonIndexAttachPackageStatusResult::WouldAttach
    ));
    assert!(matches!(
        result.packages[1].status,
        AddonIndexAttachPackageStatusResult::NoLocalMatch
    ));

    let inventory = AddonService::new()
        .list(crate::core::app::ListAddonsRequest { installation })
        .expect("list addons");
    assert!(inventory.tracked_packages[0].metadata.is_none());
}

#[test]
fn addon_index_service_attach_collecting_progress_returns_attach_task_events() {
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
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Details"]
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install details");

    let service = AddonIndexService::new();
    let run = service
        .attach_collecting_progress(AttachAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("curated-details".to_string()),
            dry_run: false,
            apply_ready_only: false,
        })
        .expect("attach from index with collected progress");

    assert!(run.result.applied);
    let phases = run
        .progress
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases.first(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::AddonIndexAttach, TaskPhase::Executing)));
}
