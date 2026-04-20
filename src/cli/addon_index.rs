use super::AddonIndexCommands;
use super::app_support::{extended_services, resolve_cli_installation};
use super::output::{
    render, render_addon_index_inspection, render_addon_index_install, render_addon_index_update,
};
use crate::core::error::AppResult;

mod request;

use request::{
    build_inspect_addon_index_request, build_install_addon_index_request,
    build_update_addon_index_request,
};

pub(super) fn handle_addon_index_command(json: bool, command: AddonIndexCommands) -> AppResult<()> {
    let app = extended_services();

    match command {
        AddonIndexCommands::Inspect { file } => {
            let inspection = app.inspect_addon_index(build_inspect_addon_index_request(file))?;
            render(json, &inspection, render_addon_index_inspection)?;
        }
        AddonIndexCommands::Install {
            install_target,
            file,
            name,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_cli_installation(app.stable(), install_target)?;
            let result = app.install_addon_index(build_install_addon_index_request(
                installation,
                file,
                name,
                dry_run,
                backup_output,
                replace_existing,
            ))?;
            render(json, &result, render_addon_index_install)?;
        }
        AddonIndexCommands::Update {
            install_target,
            file,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_cli_installation(app.stable(), install_target)?;
            let result = app.update_addon_index(build_update_addon_index_request(
                installation,
                file,
                name,
                dry_run,
                backup_output,
            ))?;
            render(json, &result, render_addon_index_update)?;
        }
    }

    Ok(())
}
