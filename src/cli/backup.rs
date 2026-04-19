use super::BackupCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::output::{render, render_backup_catalog, render_backup_created, render_backup_restored};
use crate::core::app::{
    BackupGroupValue, CreateBackupAppRequest, ListBackupsRequest, RestoreBackupAppRequest,
};
use crate::core::error::AppResult;

pub(super) fn handle_backup_command(json: bool, command: BackupCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        BackupCommands::Create {
            install,
            flavor,
            output,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
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
            render(json, &backup, render_backup_created)?;
        }
        BackupCommands::List { dir } => {
            let backups = app.list_backups(ListBackupsRequest { backup_dir: dir })?;
            render(json, &backups, render_backup_catalog)?;
        }
        BackupCommands::Restore {
            install,
            flavor,
            archive,
            id,
            dir,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let restored = app.restore_backup(RestoreBackupAppRequest {
                installation,
                archive_path: archive,
                backup_id: id,
                backup_dir: dir,
            })?;
            render(json, &restored, render_backup_restored)?;
        }
    }

    Ok(())
}
