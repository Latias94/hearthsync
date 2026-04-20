use super::AddonCommands;
use super::app_support::{render_with_installation, stable_services};
use super::output::{
    render_addon_install, render_addon_inventory, render_addon_remove, render_addon_search_catalog,
    render_addon_update,
};
use crate::core::error::{AppError, AppResult};

mod request;

use request::{
    build_install_addon_request, build_list_addons_request, build_remove_addons_request,
    build_search_addons_request, build_update_addons_request,
};

pub(super) fn handle_basic_addon_command(json: bool, command: AddonCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        AddonCommands::Search {
            install_target,
            query,
            limit,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| build_search_addons_request(installation, query, limit),
            |request| app.search_addons(request),
            render_addon_search_catalog,
        )?,
        AddonCommands::List { install_target } => render_with_installation(
            json,
            &app,
            install_target,
            build_list_addons_request,
            |request| app.list_addons(request),
            render_addon_inventory,
        )?,
        AddonCommands::Install {
            install_target,
            source,
            dry_run,
            backup_output,
            replace_existing,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| {
                build_install_addon_request(
                    installation,
                    source,
                    dry_run,
                    backup_output,
                    replace_existing,
                )
            },
            |request| app.install_addon(request),
            render_addon_install,
        )?,
        AddonCommands::Update {
            install_target,
            name,
            dry_run,
            backup_output,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| build_update_addons_request(installation, name, dry_run, backup_output),
            |request| app.update_addons(request),
            render_addon_update,
        )?,
        AddonCommands::Remove {
            install_target,
            name,
            dry_run,
            backup_output,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| build_remove_addons_request(installation, name, dry_run, backup_output),
            |request| app.remove_addons(request),
            render_addon_remove,
        )?,
        AddonCommands::Index { .. } | AddonCommands::Lock { .. } => {
            return Err(AppError::Validation(
                "internal CLI routing error: addon subcommand reached basic addon handler"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
