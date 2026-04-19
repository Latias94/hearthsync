use super::AddonIndexCommands;
use super::app_support::{extended_services, resolve_cli_installation};
use super::output::{
    render, render_addon_index_inspection, render_addon_index_install, render_addon_index_update,
};
use crate::core::app::{
    InspectAddonIndexRequest, InstallAddonIndexAppRequest, UpdateAddonIndexAppRequest,
};
use crate::core::error::AppResult;

pub(super) fn handle_addon_index_command(json: bool, command: AddonIndexCommands) -> AppResult<()> {
    let app = extended_services();

    match command {
        AddonIndexCommands::Inspect { file } => {
            let inspection =
                app.inspect_addon_index(InspectAddonIndexRequest { index_path: file })?;
            render(json, &inspection, render_addon_index_inspection)?;
        }
        AddonIndexCommands::Install {
            install,
            flavor,
            file,
            name,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.install_addon_index(InstallAddonIndexAppRequest {
                installation,
                index_path: file,
                name,
                dry_run,
                backup_output_path: backup_output,
                replace_existing,
            })?;
            render(json, &result, render_addon_index_install)?;
        }
        AddonIndexCommands::Update {
            install,
            flavor,
            file,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.update_addon_index(UpdateAddonIndexAppRequest {
                installation,
                index_path: file,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
            render(json, &result, render_addon_index_update)?;
        }
    }

    Ok(())
}
