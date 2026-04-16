use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use super::{
    BackupGroup, BackupMetadata, BackupRequest, RestoreBackupRequest, create_backup, list_backups,
    restore_backup, restore_backup_selection,
};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

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

fn write_test_backup_archive(path: &Path, metadata: BackupMetadata) {
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
    zip.finish().expect("finish archive");
}
