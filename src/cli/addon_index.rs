use super::AddonIndexCommands;
use super::app_support::{extended_services, render_with_installation, render_with_value};
use super::output::addon::{
    render_addon_index_inspection, render_addon_index_install, render_addon_index_update,
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
        AddonIndexCommands::Inspect { file } => render_with_value(
            json,
            || app.inspect_addon_index(build_inspect_addon_index_request(file)),
            render_addon_index_inspection,
        )?,
        AddonIndexCommands::Install {
            install_target,
            file,
            name,
            dry_run,
            backup_output,
            replace_existing,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| {
                build_install_addon_index_request(
                    installation,
                    file,
                    name,
                    dry_run,
                    backup_output,
                    replace_existing,
                )
            },
            |request| app.install_addon_index(request),
            render_addon_index_install,
        )?,
        AddonIndexCommands::Update {
            install_target,
            file,
            name,
            dry_run,
            backup_output,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| {
                build_update_addon_index_request(installation, file, name, dry_run, backup_output)
            },
            |request| app.update_addon_index(request),
            render_addon_index_update,
        )?,
    }

    Ok(())
}
