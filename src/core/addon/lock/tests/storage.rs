use super::*;

#[test]
fn install_addon_writes_lock_with_metadata_and_content_hash() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Title: Details!\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: Some(AddonPackageMetadata {
            index_name: Some("Fixture Index".to_string()),
            index_package_id: Some("details".to_string()),
            package_name: Some("Details".to_string()),
            version: Some("1.0.0".to_string()),
            source_url: Some("https://example.com/details.zip".to_string()),
            website_url: Some("https://example.com/details".to_string()),
            source_sha256: Some("source-hash".to_string()),
            supported_flavors: vec!["retail".to_string()],
        }),
    })
    .expect("install addon");

    let inspection =
        inspect_addon_lock(&installation, &addon_state_paths(&installation)).expect("inspect lock");
    assert_eq!(inspection.package_count, 1);
    assert_eq!(
        inspection.lock.packages[0].index_package_id.as_deref(),
        Some("details")
    );
    assert_eq!(inspection.lock.packages[0].name.as_deref(), Some("Details"));
    assert_eq!(
        inspection.lock.packages[0].version.as_deref(),
        Some("1.0.0")
    );
    assert_eq!(inspection.lock.packages[0].content_sha256.len(), 64);
    assert_eq!(
        inspection.lock.packages[0].addon_directories,
        vec!["Details"]
    );
}

#[test]
fn write_addon_lock_removes_stale_lock_when_registry_is_empty() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let path = lock_path(&addon_state_paths(&installation));
    fs::create_dir_all(path.parent().expect("lock parent")).expect("lock parent");
    fs::write(&path, "stale").expect("stale lock");

    let result =
        write_addon_lock(&installation, &addon_state_paths(&installation)).expect("write lock");

    assert!(result.removed);
    assert!(!path.exists());
}

#[test]
fn remove_addon_cleans_lock_file_when_last_package_is_removed() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");
    assert!(lock_path(&addon_state_paths(&installation)).exists());

    remove_addons(RemoveAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: "Details".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("remove addon");

    assert!(!lock_path(&addon_state_paths(&installation)).exists());
}
