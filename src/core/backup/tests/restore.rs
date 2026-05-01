use super::*;

#[test]
fn create_and_restore_backup_preserve_large_binary_file_contents() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), WowFlavor::Retail);
    let addon_root = installation.addon_dir.join("WeakAuras");
    fs::create_dir_all(&addon_root).expect("addon dir");

    let original = (0..262_144usize)
        .map(|index| (index % 251) as u8)
        .collect::<Vec<_>>();
    fs::write(addon_root.join("WeakAuras.dat"), &original).expect("write binary addon file");

    let backup = create_backup(BackupRequest {
        installation: installation.clone(),
        output_path: Some(temp.path().join("out")),
        groups: vec![BackupGroup::Addons],
        label: Some("large-binary".to_string()),
    })
    .expect("create backup");

    fs::write(addon_root.join("WeakAuras.dat"), vec![0u8; original.len()])
        .expect("overwrite binary addon file");

    let restored = restore_backup(&backup.archive_path, &installation).expect("restore");

    assert_eq!(restored.restored_files, 1);
    assert_eq!(
        fs::read(addon_root.join("WeakAuras.dat")).expect("read restored binary"),
        original
    );
}

#[test]
fn restore_backup_restores_previous_state_and_removes_new_files() {
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
        fonts_dir: fonts_dir.clone(),
    };

    fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");
    fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "before").expect("toc");
    fs::write(wtf_dir.join("Config.wtf"), "before").expect("config");

    let backup = create_backup(BackupRequest {
        installation: installation.clone(),
        output_path: Some(temp.path().join("out")),
        groups: vec![BackupGroup::Addons, BackupGroup::Wtf],
        label: Some("rollback".to_string()),
    })
    .expect("backup");

    fs::write(addon_dir.join("WeakAuras").join("WeakAuras.toc"), "after").expect("toc");
    fs::write(wtf_dir.join("Config.wtf"), "after").expect("config");
    fs::write(wtf_dir.join("New.lua"), "new").expect("new file");

    let restored = restore_backup(&backup.archive_path, &installation).expect("restore");

    assert_eq!(restored.metadata.groups.len(), 2);
    assert_eq!(
        fs::read_to_string(addon_dir.join("WeakAuras").join("WeakAuras.toc")).expect("toc"),
        "before"
    );
    assert_eq!(
        fs::read_to_string(wtf_dir.join("Config.wtf")).expect("config"),
        "before"
    );
    assert!(!wtf_dir.join("New.lua").exists());
}

#[test]
fn restore_backup_rejects_flavor_mismatch_without_touching_target() {
    let temp = tempdir().expect("temp dir");
    let retail = create_fixture_installation(temp.path(), WowFlavor::Retail);
    let classic = create_fixture_installation(&temp.path().join("classic"), WowFlavor::Classic);

    fs::create_dir_all(retail.addon_dir.join("WeakAuras")).expect("retail addon dir");
    fs::write(
        retail.addon_dir.join("WeakAuras").join("WeakAuras.toc"),
        "retail-before",
    )
    .expect("retail toc");
    fs::create_dir_all(classic.addon_dir.join("Questie")).expect("classic addon dir");
    fs::write(
        classic.addon_dir.join("Questie").join("Questie.toc"),
        "classic-current",
    )
    .expect("classic toc");

    let backup = create_backup(BackupRequest {
        installation: retail,
        output_path: Some(temp.path().join("out")),
        groups: vec![BackupGroup::Addons],
        label: Some("retail-only".to_string()),
    })
    .expect("backup");

    let error = restore_backup(&backup.archive_path, &classic).expect_err("flavor mismatch");
    assert!(matches!(error, crate::core::error::AppError::Validation(_)));
    assert!(
        error
            .to_string()
            .contains("does not match target flavor `classic`")
    );
    assert_eq!(
        fs::read_to_string(classic.addon_dir.join("Questie").join("Questie.toc"))
            .expect("classic toc"),
        "classic-current"
    );
}

#[test]
fn restore_backup_rolls_back_to_pre_restore_state_when_apply_fails() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), WowFlavor::Retail);

    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::create_dir_all(&installation.wtf_dir).expect("wtf dir");
    fs::write(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "before-addon",
    )
    .expect("before addon");
    fs::write(installation.wtf_dir.join("Config.wtf"), "before-wtf").expect("before wtf");

    let backup = create_backup(BackupRequest {
        installation: installation.clone(),
        output_path: Some(temp.path().join("out")),
        groups: vec![BackupGroup::Addons, BackupGroup::Wtf],
        label: Some("transaction".to_string()),
    })
    .expect("backup");

    fs::write(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "after-addon",
    )
    .expect("after addon");
    fs::write(installation.wtf_dir.join("Config.wtf"), "after-wtf").expect("after wtf");
    fs::write(installation.wtf_dir.join("New.lua"), "new-wtf").expect("new wtf");

    crate::core::backup::archive::set_restore_test_failure_after(Some(1));
    let error =
        restore_backup(&backup.archive_path, &installation).expect_err("restore should fail");
    crate::core::backup::archive::set_restore_test_failure_after(None);

    assert!(matches!(error, crate::core::error::AppError::Validation(_)));
    assert!(
        error
            .to_string()
            .contains("transactional rollback restored pre-restore state")
    );
    assert_eq!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("addon toc"),
        "after-addon"
    );
    assert_eq!(
        fs::read_to_string(installation.wtf_dir.join("Config.wtf")).expect("wtf"),
        "after-wtf"
    );
    assert_eq!(
        fs::read_to_string(installation.wtf_dir.join("New.lua")).expect("new wtf"),
        "new-wtf"
    );
}

#[test]
fn restore_backup_rejects_symlink_entries_without_touching_target() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), WowFlavor::Retail);
    let archive_path = temp.path().join("symlink-backup.zip");

    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "before-restore",
    )
    .expect("write addon");

    write_test_backup_archive_with_entries(
        &archive_path,
        BackupMetadata {
            schema_version: 1,
            created_at: "2026-04-20T12:00:00Z".to_string(),
            label: Some("symlink".to_string()),
            flavor: installation.flavor.as_str().to_string(),
            flavor_root: installation.flavor_root.clone(),
            groups: vec![BackupGroup::Addons],
        },
        &[TestBackupArchiveEntry::Symlink {
            name: "addons/WeakAuras/WeakAuras.toc",
            target: "../Elsewhere/WeakAuras.toc",
        }],
    );

    let error = restore_backup(&archive_path, &installation).expect_err("symlink should fail");
    let message = error.to_string();
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
    assert_eq!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("addon toc"),
        "before-restore"
    );
}

#[test]
fn restore_backup_rejects_non_portable_archive_paths() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), WowFlavor::Retail);

    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "before-restore",
    )
    .expect("write addon");

    for (archive_name, archive_file_name) in [
        (
            "addons//WeakAuras/WeakAuras.toc",
            "backup-invalid-empty-segment.zip",
        ),
        (
            "addons/Weak:Auras/WeakAuras.toc",
            "backup-invalid-reserved-char.zip",
        ),
        ("addons/CON/WeakAuras.toc", "backup-invalid-device-name.zip"),
    ] {
        let archive_path = temp.path().join(archive_file_name);
        write_test_backup_archive_with_entries(
            &archive_path,
            BackupMetadata {
                schema_version: 1,
                created_at: "2026-04-20T12:00:00Z".to_string(),
                label: Some("unsafe-path".to_string()),
                flavor: installation.flavor.as_str().to_string(),
                flavor_root: installation.flavor_root.clone(),
                groups: vec![BackupGroup::Addons],
            },
            &[TestBackupArchiveEntry::File {
                name: archive_name,
                content: "## Interface: 110000",
            }],
        );

        let error = restore_backup(&archive_path, &installation)
            .expect_err("non-portable archive path should fail");
        assert!(matches!(error, crate::core::error::AppError::Validation(_)));
        assert!(error.to_string().contains("unsafe archive path"));
        assert_eq!(
            fs::read_to_string(
                installation
                    .addon_dir
                    .join("WeakAuras")
                    .join("WeakAuras.toc")
            )
            .expect("addon toc"),
            "before-restore"
        );
    }
}

#[test]
fn restore_backup_rejects_case_insensitive_restore_destination_collisions() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), WowFlavor::Retail);
    let archive_path = temp.path().join("case-collision-backup.zip");

    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "before-restore",
    )
    .expect("write addon");

    write_test_backup_archive_with_entries(
        &archive_path,
        BackupMetadata {
            schema_version: 1,
            created_at: "2026-04-21T08:00:00Z".to_string(),
            label: Some("case-collision".to_string()),
            flavor: installation.flavor.as_str().to_string(),
            flavor_root: installation.flavor_root.clone(),
            groups: vec![BackupGroup::Addons],
        },
        &[
            TestBackupArchiveEntry::File {
                name: "addons/WeakAuras/Config.lua",
                content: "first",
            },
            TestBackupArchiveEntry::File {
                name: "addons/weakauras/config.lua",
                content: "second",
            },
        ],
    );

    let error = restore_backup(&archive_path, &installation)
        .expect_err("case-insensitive destinations should fail");
    let message = error.to_string();
    assert!(matches!(error, crate::core::error::AppError::Validation(_)));
    assert!(message.contains("case-insensitive restore destination collisions"));
    assert!(message.contains("addons/WeakAuras/Config.lua"));
    assert!(message.contains("addons/weakauras/config.lua"));
    assert_eq!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("addon toc"),
        "before-restore"
    );
}

#[test]
fn restore_backup_rejects_case_insensitive_restore_prefix_conflicts() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), WowFlavor::Retail);
    let archive_path = temp.path().join("case-prefix-collision-backup.zip");

    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "before-restore",
    )
    .expect("write addon");

    write_test_backup_archive_with_entries(
        &archive_path,
        BackupMetadata {
            schema_version: 1,
            created_at: "2026-04-21T08:00:00Z".to_string(),
            label: Some("case-prefix-collision".to_string()),
            flavor: installation.flavor.as_str().to_string(),
            flavor_root: installation.flavor_root.clone(),
            groups: vec![BackupGroup::Addons],
        },
        &[
            TestBackupArchiveEntry::File {
                name: "addons/WeakAuras",
                content: "root-file",
            },
            TestBackupArchiveEntry::File {
                name: "addons/weakauras/Config.lua",
                content: "nested-file",
            },
        ],
    );

    let error = restore_backup(&archive_path, &installation)
        .expect_err("case-insensitive file/directory conflicts should fail");
    let message = error.to_string();
    assert!(matches!(error, crate::core::error::AppError::Validation(_)));
    assert!(message.contains("case-insensitive conflicting restore destinations"));
    assert!(message.contains("addons/WeakAuras"));
    assert!(message.contains("addons/weakauras/Config.lua"));
    assert_eq!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("addon toc"),
        "before-restore"
    );
}
