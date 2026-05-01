use super::*;

#[test]
fn addon_service_adopt_tracks_explicit_untracked_addon_as_local_snapshot() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let addon_dir = installation.addon_dir.join("Plater");
    fs::create_dir_all(&addon_dir).expect("plater dir");
    fs::write(
        addon_dir.join("Plater.toc"),
        "## Interface: 110000\n## Title: Plater\n",
    )
    .expect("write toc");

    let service = AddonService::new();
    let adopted = service
        .adopt(AdoptAddonsAppRequest {
            installation: installation.clone(),
            addon_directories: vec!["Plater".to_string()],
            package_id: None,
            archive_output_path: None,
            dry_run: false,
        })
        .expect("adopt addon");
    let inventory = service
        .list(ListAddonsRequest { installation })
        .expect("list addons");

    assert_eq!(adopted.package_id, "plater");
    assert_eq!(
        adopted.source.kind,
        crate::core::app::AddonSourceKindResult::LocalArchive
    );
    assert_eq!(adopted.addon_count, 1);
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert!(inventory.untracked_addons.is_empty());
}

#[test]
fn addon_service_relink_updates_tracked_source_without_reinstalling_files() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let installed_archive = temp.path().join("Details.zip");
    let relink_archive = temp.path().join("Details-release.zip");
    create_addon_archive(
        &installed_archive,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &relink_archive,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );

    let service = AddonService::new();
    service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: installed_archive.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: Some(AddonPackageMetadataValue {
                index_name: Some("curated".to_string()),
                index_package_id: Some("details".to_string()),
                package_name: Some("Details".to_string()),
                version: Some("1.0.0".to_string()),
                source_url: Some("https://example.invalid/details.zip".to_string()),
                website_url: Some("https://example.invalid/details".to_string()),
                source_sha256: Some("abc123".to_string()),
                supported_flavors: vec!["retail".to_string()],
            }),
        })
        .expect("install addon");

    let relinked = service
        .relink(RelinkAddonAppRequest {
            installation: installation.clone(),
            name: "Details".to_string(),
            source: relink_archive.display().to_string(),
            dry_run: false,
        })
        .expect("relink addon source");
    let inventory = service
        .list(ListAddonsRequest {
            installation: installation.clone(),
        })
        .expect("list addons");

    assert_eq!(relinked.package_id, "details");
    assert!(relinked.cleared_metadata);
    assert_eq!(
        relinked.source.local_archive_path,
        Some(
            crate::core::addon::canonicalize_local_archive_path(&relink_archive)
                .expect("normalized relink archive")
        )
    );
    assert!(inventory.tracked_packages[0].metadata.is_none());
    assert_eq!(
        inventory.tracked_packages[0].source.local_archive_path,
        Some(
            crate::core::addon::canonicalize_local_archive_path(&relink_archive)
                .expect("normalized relink archive")
        )
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("installed toc")
            .contains("1.0.0")
    );
}

#[test]
fn addon_service_install_and_list_roundtrip_app_owned_metadata() {
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
            metadata: Some(AddonPackageMetadataValue {
                index_name: Some("curated".to_string()),
                index_package_id: Some("details".to_string()),
                package_name: Some("Details".to_string()),
                version: Some("1.0.0".to_string()),
                source_url: Some("https://example.invalid/details.zip".to_string()),
                website_url: Some("https://example.invalid/details".to_string()),
                source_sha256: Some("abc123".to_string()),
                supported_flavors: vec!["retail".to_string()],
            }),
        })
        .expect("install addon");

    let inventory = service
        .list(ListAddonsRequest { installation })
        .expect("list addons");
    let metadata = inventory.tracked_packages[0]
        .metadata
        .as_ref()
        .expect("tracked metadata");

    assert_eq!(metadata.index_name.as_deref(), Some("curated"));
    assert_eq!(metadata.index_package_id.as_deref(), Some("details"));
    assert_eq!(metadata.package_name.as_deref(), Some("Details"));
    assert_eq!(metadata.version.as_deref(), Some("1.0.0"));
    assert_eq!(
        metadata.source_url.as_deref(),
        Some("https://example.invalid/details.zip")
    );
    assert_eq!(metadata.supported_flavors, vec!["retail"]);
}
