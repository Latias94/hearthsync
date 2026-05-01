use super::*;

#[test]
fn create_backup_writes_expected_entries() {
    let temp = tempdir().expect("temp dir");
    let flavor_root = temp.path().join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
    fs::create_dir_all(wtf_dir.join("Account")).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");
    fs::write(
        addon_dir.join("WeakAuras").join("WeakAuras.toc"),
        "## Interface: 110000",
    )
    .expect("toc");
    fs::write(wtf_dir.join("Config.wtf"), "SET locale enUS").expect("config");
    fs::write(fonts_dir.join("FRIZQT__.ttf"), "font").expect("font");

    let backup = create_backup(BackupRequest {
        installation: DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root: temp.path().to_path_buf(),
            flavor_root: flavor_root.clone(),
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        },
        output_path: Some(temp.path().join("out")),
        groups: vec![BackupGroup::Addons, BackupGroup::Wtf, BackupGroup::Fonts],
        label: Some("smoke".to_string()),
    })
    .expect("backup");

    let file = std::fs::File::open(backup.archive_path).expect("archive");
    let mut archive = ZipArchive::new(file).expect("zip");

    assert!(archive.by_name("addons/WeakAuras/WeakAuras.toc").is_ok());
    assert!(archive.by_name("wtf/Config.wtf").is_ok());
    assert!(archive.by_name("fonts/FRIZQT__.ttf").is_ok());
    assert!(archive.by_name("backup.toml").is_ok());
}

#[test]
fn create_backup_rejects_non_portable_label_before_creating_output_dir() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), WowFlavor::Retail);
    let output_dir = temp.path().join("out");

    let error = create_backup(BackupRequest {
        installation,
        output_path: Some(output_dir.clone()),
        groups: vec![BackupGroup::Addons],
        label: Some("../escape".to_string()),
    })
    .expect_err("path-shaped label should fail");

    assert!(error.to_string().contains("invalid backup label name"));
    assert!(!output_dir.exists());
}

#[test]
fn reject_unsupported_backup_source_symlink_reports_directory_entries() {
    let error = crate::core::backup::archive::reject_unsupported_backup_source_symlink(
        "directory",
        Path::new("Interface/AddOns/WeakAuras"),
        true,
    )
    .expect_err("directory symlink should fail");

    let message = error.to_string();
    assert!(message.contains("backup directory entry"));
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("Interface/AddOns/WeakAuras"));
}

#[test]
fn reject_unsupported_backup_source_symlink_allows_regular_entries() {
    crate::core::backup::archive::reject_unsupported_backup_source_symlink(
        "interface asset",
        Path::new("Interface/SharedMedia"),
        false,
    )
    .expect("regular entry should pass");
}
