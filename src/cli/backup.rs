use super::BackupCommands;
use super::output::render;
use crate::core::app::BackupService;
use crate::core::backup::{BackupGroup, BackupRequest, RestoreBackupRequest};
use crate::core::error::AppResult;
use crate::core::install::resolve_installation;

pub(super) fn handle_backup_command(json: bool, command: BackupCommands) -> AppResult<()> {
    let service = BackupService::new();

    match command {
        BackupCommands::Create {
            install,
            flavor,
            output,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let backup = service.create(BackupRequest {
                installation,
                output_path: output,
                groups: vec![
                    BackupGroup::Addons,
                    BackupGroup::Wtf,
                    BackupGroup::Fonts,
                    BackupGroup::InterfaceAssets,
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
                        .map(BackupGroup::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        }
        BackupCommands::List { dir } => {
            let backups = service.list(dir.as_deref())?;
            render(json, &backups, |item| {
                if item.entries.is_empty() {
                    format!(
                        "Backup dir: {}\nNo backups found.",
                        item.backup_dir.display()
                    )
                } else {
                    let mut lines = vec![
                        format!("Backup dir: {}", item.backup_dir.display()),
                        format!("Found {} backup(s):", item.entries.len()),
                    ];
                    for entry in &item.entries {
                        let groups = entry
                            .metadata
                            .groups
                            .iter()
                            .map(BackupGroup::as_str)
                            .collect::<Vec<_>>()
                            .join(", ");
                        let label = entry.metadata.label.as_deref().unwrap_or("none");
                        lines.push(format!(
                            "- {} | label: {} | created: {} | flavor: {} | groups: {} | size: {} bytes | path: {}",
                            entry.backup_id,
                            label,
                            entry.metadata.created_at,
                            entry.metadata.flavor,
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
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let restored = service.restore(RestoreBackupRequest {
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
                        .map(BackupGroup::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;
        }
    }

    Ok(())
}
