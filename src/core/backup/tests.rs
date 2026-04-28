use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use super::{
    BackupGroup, BackupMetadata, BackupRequest, RestoreBackupRequest, create_backup, list_backups,
    restore_backup, restore_backup_selection, restore_backup_selection_task,
};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::{NeverCancel, TaskKind, TaskPhase, TaskProgressEvent, VecTaskProgressSink};

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
fn list_backups_reads_metadata_and_sorts_newest_first() {
    let temp = tempdir().expect("temp dir");
    let backup_dir = temp.path().join("backups");
    fs::create_dir_all(&backup_dir).expect("backup dir");

    write_test_backup_archive(
        &backup_dir.join("backup-retail-old.zip"),
        BackupMetadata {
            schema_version: 1,
            created_at: "2026-04-15T10:00:00Z".to_string(),
            label: Some("old".to_string()),
            flavor: "retail".to_string(),
            flavor_root: PathBuf::from("C:/WoW/_retail_"),
            groups: vec![BackupGroup::Addons],
        },
    );
    write_test_backup_archive(
        &backup_dir.join("backup-retail-new.zip"),
        BackupMetadata {
            schema_version: 1,
            created_at: "2026-04-15T11:00:00Z".to_string(),
            label: Some("new".to_string()),
            flavor: "retail".to_string(),
            flavor_root: PathBuf::from("C:/WoW/_retail_"),
            groups: vec![BackupGroup::Wtf, BackupGroup::Fonts],
        },
    );

    let catalog = list_backups(Some(&backup_dir)).expect("list backups");

    assert_eq!(catalog.entries.len(), 2);
    assert_eq!(catalog.entries[0].backup_id, "backup-retail-new");
    assert_eq!(catalog.entries[0].metadata.label.as_deref(), Some("new"));
    assert_eq!(catalog.entries[1].backup_id, "backup-retail-old");
    assert_eq!(catalog.entries[1].metadata.label.as_deref(), Some("old"));
    assert!(
        catalog
            .entries
            .iter()
            .all(|entry| entry.archive_size_bytes > 0)
    );
}

#[test]
fn resolve_backup_dir_defaults_to_clean_appdata_layout() {
    let backup_dir = super::storage::resolve_backup_dir(None).expect("default backup dir");
    let path = backup_dir.to_string_lossy().replace('\\', "/");

    assert!(path.ends_with("/hearthsync/data/backups") || path.ends_with("/hearthsync/backups"));
    assert!(!path.contains("/hearthsync/hearthsync/"));
}

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

    super::archive::set_restore_test_failure_after(Some(1));
    let error =
        restore_backup(&backup.archive_path, &installation).expect_err("restore should fail");
    super::archive::set_restore_test_failure_after(None);

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

#[test]
fn reject_unsupported_backup_source_symlink_reports_directory_entries() {
    let error = super::archive::reject_unsupported_backup_source_symlink(
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
    super::archive::reject_unsupported_backup_source_symlink(
        "interface asset",
        Path::new("Interface/SharedMedia"),
        false,
    )
    .expect("regular entry should pass");
}

enum TestBackupArchiveEntry<'a> {
    File { name: &'a str, content: &'a str },
    Symlink { name: &'a str, target: &'a str },
}

fn write_test_backup_archive(path: &Path, metadata: BackupMetadata) {
    write_test_backup_archive_with_entries(path, metadata, &[]);
}

fn write_test_backup_archive_with_entries(
    path: &Path,
    metadata: BackupMetadata,
    entries: &[TestBackupArchiveEntry<'_>],
) {
    let file = File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    zip.start_file("backup.toml", SimpleFileOptions::default())
        .expect("start backup metadata");
    zip.write_all(
        toml::to_string_pretty(&metadata)
            .expect("serialize metadata")
            .as_bytes(),
    )
    .expect("write backup metadata");
    for entry in entries {
        match entry {
            TestBackupArchiveEntry::File { name, content } => {
                zip.start_file(*name, SimpleFileOptions::default())
                    .expect("start backup entry");
                zip.write_all(content.as_bytes())
                    .expect("write backup entry");
            }
            TestBackupArchiveEntry::Symlink { name, target } => {
                zip.add_symlink(*name, *target, SimpleFileOptions::default())
                    .expect("add backup symlink entry");
            }
        }
    }
    zip.finish().expect("finish archive");
}

fn create_fixture_installation(root: &Path, flavor: WowFlavor) -> DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join(flavor.folder_name());
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root,
        flavor_root,
        flavor,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn assert_backup_restore_task_progress(events: &[TaskProgressEvent]) {
    let phases = events
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();

    assert_eq!(
        phases.first(),
        Some(&(TaskKind::BackupRestore, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::BackupRestore, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::BackupRestore, TaskPhase::BackingUp)));
    assert!(
        phases
            .iter()
            .any(|phase| { *phase == (TaskKind::BackupRestore, TaskPhase::Executing) })
    );
    assert!(events.iter().any(|event| {
        event.task == TaskKind::BackupRestore
            && event.phase == TaskPhase::Executing
            && (event.message.contains("Clearing restore target group")
                || event.message.contains("Restoring backup entry"))
    }));
}
