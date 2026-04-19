use super::BackupCommands;
use super::output::render;
use crate::core::app::{
    BackupGroupValue, CreateBackupAppRequest, HearthSyncApp, ListBackupsRequest,
    ResolveInstallationRequest, RestoreBackupAppRequest,
};
use crate::core::error::AppResult;

pub(super) fn handle_backup_command(json: bool, command: BackupCommands) -> AppResult<()> {
    let app = HearthSyncApp::new();

    match command {
        BackupCommands::Create {
            install,
            flavor,
            output,
        } => {
            let installation = app.resolve_installation(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let backup = app.create_backup(CreateBackupAppRequest {
                installation,
                output_path: output,
                groups: vec![
                    BackupGroupValue::Addons,
                    BackupGroupValue::Wtf,
                    BackupGroupValue::Fonts,
                    BackupGroupValue::InterfaceAssets,
                ],
                label: None,
            })?;
            render(json, &backup, |item| {
                format!(
                    "Created backup: {}\nArchived files: {}\nGroups: {}",
                    item.archive_path.display(),
                    item.archived_files,
                    item.metadata
                        .groups
                        .iter()
                        .map(format_backup_group)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        }
        BackupCommands::List { dir } => {
            let backups = app.list_backups(ListBackupsRequest { backup_dir: dir })?;
            render(json, &backups, |item| {
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
                        let groups = entry
                            .groups
                            .iter()
                            .map(format_backup_group)
                            .collect::<Vec<_>>()
                            .join(", ");
                        let label = entry.label.as_deref().unwrap_or("none");
                        lines.push(format!(
                            "- {} | label: {} | created: {} | flavor: {} | groups: {} | size: {} bytes | path: {}",
                            entry.backup_id,
                            label,
                            entry.created_at,
                            entry.flavor,
                            groups,
                            entry.archive_size_bytes,
                            entry.archive_path.display()
                        ));
                    }
                    lines.join("\n")
                }
            })?;
        }
        BackupCommands::Restore {
            install,
            flavor,
            archive,
            id,
            dir,
        } => {
            let installation = app.resolve_installation(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let restored = app.restore_backup(RestoreBackupAppRequest {
                installation,
                archive_path: archive,
                backup_id: id,
                backup_dir: dir,
            })?;
            render(json, &restored, |item| {
                format!(
                    "Restored backup: {}\nRestored files: {}\nCreated at: {}\nLabel: {}\nGroups: {}",
                    item.archive_path.display(),
                    item.restored_files,
                    item.metadata.created_at,
                    item.metadata.label.as_deref().unwrap_or("none"),
                    item.metadata
                        .groups
                        .iter()
                        .map(format_backup_group)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        }
    }

    Ok(())
}

fn format_backup_group(group: &BackupGroupValue) -> &'static str {
    match group {
        BackupGroupValue::Addons => "addons",
        BackupGroupValue::Wtf => "wtf",
        BackupGroupValue::Fonts => "fonts",
        BackupGroupValue::InterfaceAssets => "interface_assets",
    }
}
