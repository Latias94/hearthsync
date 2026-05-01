use super::*;

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
fn list_backups_rejects_invalid_metadata_contracts() {
    let temp = tempdir().expect("temp dir");
    let backup_dir = temp.path().join("backups");
    fs::create_dir_all(&backup_dir).expect("backup dir");

    write_test_backup_archive(
        &backup_dir.join("backup-invalid-created-at.zip"),
        BackupMetadata {
            schema_version: 1,
            created_at: "not-a-timestamp".to_string(),
            label: Some("invalid".to_string()),
            flavor: "retail".to_string(),
            flavor_root: PathBuf::from("C:/WoW/_retail_"),
            groups: vec![BackupGroup::Addons],
        },
    );

    let error = list_backups(Some(&backup_dir)).expect_err("invalid metadata should fail closed");

    assert!(
        error
            .to_string()
            .contains("backup metadata created_at must be an RFC 3339 timestamp")
    );
}

#[test]
fn list_backups_rejects_non_portable_metadata_label() {
    let temp = tempdir().expect("temp dir");
    let backup_dir = temp.path().join("backups");
    fs::create_dir_all(&backup_dir).expect("backup dir");

    write_test_backup_archive(
        &backup_dir.join("backup-invalid-label.zip"),
        BackupMetadata {
            schema_version: 1,
            created_at: "2026-04-15T10:00:00Z".to_string(),
            label: Some("../escape".to_string()),
            flavor: "retail".to_string(),
            flavor_root: PathBuf::from("C:/WoW/_retail_"),
            groups: vec![BackupGroup::Addons],
        },
    );

    let error = list_backups(Some(&backup_dir)).expect_err("invalid label should fail closed");

    assert!(error.to_string().contains("invalid backup label name"));
}

#[test]
fn list_backups_rejects_symlink_metadata_entries() {
    let temp = tempdir().expect("temp dir");
    let backup_dir = temp.path().join("backups");
    fs::create_dir_all(&backup_dir).expect("backup dir");
    write_test_backup_archive_with_symlink_metadata(
        &backup_dir.join("backup-symlink-metadata.zip"),
        "../backup.toml",
    );

    let error = list_backups(Some(&backup_dir)).expect_err("symlink metadata should fail closed");
    let message = error.to_string();
    assert!(message.contains("backup metadata entry"));
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("backup.toml"));
}

#[test]
fn resolve_backup_dir_defaults_to_clean_appdata_layout() {
    let backup_dir =
        crate::core::backup::storage::resolve_backup_dir(None).expect("default backup dir");
    let path = backup_dir.to_string_lossy().replace('\\', "/");

    assert!(path.ends_with("/hearthsync/data/backups") || path.ends_with("/hearthsync/backups"));
    assert!(!path.contains("/hearthsync/hearthsync/"));
}
