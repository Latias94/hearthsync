use super::BackupCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::output::{render, render_backup_catalog, render_backup_created, render_backup_restored};
use crate::core::error::AppResult;

mod request;

use request::{
    build_create_backup_request, build_list_backups_request, build_restore_backup_request,
};

pub(super) fn handle_backup_command(json: bool, command: BackupCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        BackupCommands::Create {
            install,
            flavor,
            output,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let backup = app.create_backup(build_create_backup_request(installation, output))?;
            render(json, &backup, render_backup_created)?;
        }
        BackupCommands::List { dir } => {
            let backups = app.list_backups(build_list_backups_request(dir))?;
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
            let restored =
                app.restore_backup(build_restore_backup_request(installation, archive, id, dir))?;
            render(json, &restored, render_backup_restored)?;
        }
    }

    Ok(())
}
