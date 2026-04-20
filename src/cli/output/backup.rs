use crate::core::app::{
    BackupCatalogResult, BackupGroupValue, CreatedBackupResult, RestoredBackupResult,
};

pub(in crate::cli) fn render_backup_created(item: &CreatedBackupResult) -> String {
    format!(
        "Created backup: {}\nArchived files: {}\nGroups: {}",
        item.archive_path.display(),
        item.archived_files,
        format_backup_groups(&item.metadata.groups)
    )
}

pub(in crate::cli) fn render_backup_catalog(item: &BackupCatalogResult) -> String {
    if item.entries.is_empty() {
        format!(
            "Backup dir: {}\nNo backups found.",
            item.backup_dir.display()
        )
    } else {
        let mut lines = vec![
            format!("Backup dir: {}", item.backup_dir.display()),
            format!("Found {} backup(s):", item.entry_count),
        ];
        for entry in &item.entries {
            let label = entry.label.as_deref().unwrap_or("none");
            lines.push(format!(
                "- {} | label: {} | created: {} | flavor: {} | groups: {} | size: {} bytes | path: {}",
                entry.backup_id,
                label,
                entry.created_at,
                entry.flavor,
                format_backup_groups(&entry.groups),
                entry.archive_size_bytes,
                entry.archive_path.display()
            ));
        }
        lines.join("\n")
    }
}

pub(in crate::cli) fn render_backup_restored(item: &RestoredBackupResult) -> String {
    format!(
        "Restored backup: {}\nRestored files: {}\nCreated at: {}\nLabel: {}\nGroups: {}",
        item.archive_path.display(),
        item.restored_files,
        item.metadata.created_at,
        item.metadata.label.as_deref().unwrap_or("none"),
        format_backup_groups(&item.metadata.groups)
    )
}

fn format_backup_groups(groups: &[BackupGroupValue]) -> String {
    groups
        .iter()
        .map(format_backup_group)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_backup_group(group: &BackupGroupValue) -> &'static str {
    match group {
        BackupGroupValue::Addons => "addons",
        BackupGroupValue::Wtf => "wtf",
        BackupGroupValue::Fonts => "fonts",
        BackupGroupValue::InterfaceAssets => "interface_assets",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::sample_backup_metadata;
    use super::*;
    use crate::core::app::{
        BackupCatalogResult, BackupEntryResult, BackupGroupValue, CreatedBackupResult,
        RestoredBackupResult,
    };

    #[test]
    fn render_backup_catalog_lists_entries() {
        let rendered = render_backup_catalog(&BackupCatalogResult {
            backup_dir: PathBuf::from("backups"),
            entry_count: 1,
            entries: vec![BackupEntryResult {
                backup_id: "backup-1".to_string(),
                archive_path: PathBuf::from("backups/backup-1.zip"),
                archive_size_bytes: 1024,
                created_at: "2026-04-19T12:00:00Z".to_string(),
                label: Some("before apply".to_string()),
                flavor: "retail".to_string(),
                flavor_root: PathBuf::from("C:\\Games\\World of Warcraft\\_retail_"),
                groups: vec![BackupGroupValue::Addons, BackupGroupValue::Wtf],
            }],
        });

        assert!(rendered.contains("Backup dir: backups"));
        assert!(rendered.contains("Found 1 backup(s):"));
        assert!(rendered.contains("backup-1 | label: before apply"));
        assert!(rendered.contains("groups: addons, wtf"));
    }

    #[test]
    fn render_backup_created_and_restored_report_groups() {
        let metadata = sample_backup_metadata();

        let created = render_backup_created(&CreatedBackupResult {
            archive_path: PathBuf::from("backup.zip"),
            archived_files: 12,
            metadata: metadata.clone(),
        });
        let restored = render_backup_restored(&RestoredBackupResult {
            archive_path: PathBuf::from("backup.zip"),
            restored_files: 10,
            metadata,
        });

        assert!(created.contains("Created backup: backup.zip"));
        assert!(created.contains("Groups: addons, wtf"));
        assert!(restored.contains("Restored backup: backup.zip"));
        assert!(restored.contains("Restored files: 10"));
        assert!(restored.contains("Groups: addons, wtf"));
    }
}
