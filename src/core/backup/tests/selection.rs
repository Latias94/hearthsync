use super::*;

#[test]
fn restore_backup_selection_resolves_backup_by_id() {
    let temp = tempdir().expect("temp dir");
    let flavor_root = temp.path().join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");
    let installation = DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root: temp.path().to_path_buf(),
        flavor_root: flavor_root.clone(),
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir: addon_dir.clone(),
        wtf_dir: wtf_dir.clone(),
        fonts_dir,
    };

    fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "before").expect("toc");

    let backup = create_backup(BackupRequest {
        installation: installation.clone(),
        output_path: Some(temp.path().join("out")),
        groups: vec![BackupGroup::Addons],
        label: Some("smoke".to_string()),
    })
    .expect("backup");

    fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "after").expect("toc");
    let backup_id = backup
        .archive_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("backup id")
        .to_string();

    let restored = restore_backup_selection(RestoreBackupRequest {
        installation,
        archive_path: None,
        backup_id: Some(backup_id),
        backup_dir: Some(temp.path().join("out")),
    })
    .expect("restore by id");

    assert_eq!(restored.metadata.label.as_deref(), Some("smoke"));
    assert_eq!(
        fs::read_to_string(addon_dir.join("WeakAuras").join("WeakAuras.toc")).expect("toc"),
        "before"
    );
}

#[test]
fn restore_backup_selection_task_reports_progress() {
    let temp = tempdir().expect("temp dir");
    let flavor_root = temp.path().join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");
    let installation = DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root: temp.path().to_path_buf(),
        flavor_root: flavor_root.clone(),
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir: addon_dir.clone(),
        wtf_dir: wtf_dir.clone(),
        fonts_dir,
    };

    fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "before").expect("toc");

    let backup = create_backup(BackupRequest {
        installation: installation.clone(),
        output_path: Some(temp.path().join("out")),
        groups: vec![BackupGroup::Addons],
        label: Some("task".to_string()),
    })
    .expect("backup");

    fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "after").expect("toc");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let restored = restore_backup_selection_task(
        RestoreBackupRequest {
            installation,
            archive_path: Some(backup.archive_path),
            backup_id: None,
            backup_dir: None,
        },
        &cancellation,
        &mut progress,
    )
    .expect("restore task");

    assert_eq!(restored.restored_files, 1);
    assert_backup_restore_task_progress(progress.events());
}
