use super::BackupCommands;
use super::app_support::{render_with_installation, render_with_value, stable_services};
use super::output::{render_backup_catalog, render_backup_created, render_backup_restored};
use crate::core::error::AppResult;

mod request;

use request::{
    build_create_backup_request, build_list_backups_request, build_restore_backup_request,
};

pub(super) fn handle_backup_command(json: bool, command: BackupCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        BackupCommands::Create {
            install_target,
            output,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| build_create_backup_request(installation, output),
            |request| app.create_backup(request),
            render_backup_created,
        )?,
        BackupCommands::List { dir } => render_with_value(
            json,
            || app.list_backups(build_list_backups_request(dir)),
            render_backup_catalog,
        )?,
        BackupCommands::Restore {
            install_target,
            archive,
            id,
            dir,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| build_restore_backup_request(installation, archive, id, dir),
            |request| app.restore_backup(request),
            render_backup_restored,
        )?,
    }

    Ok(())
}
