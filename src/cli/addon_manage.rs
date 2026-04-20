use super::AddonCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::output::{
    render, render_addon_install, render_addon_inventory, render_addon_remove,
    render_addon_search_catalog, render_addon_update,
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
            install,
            flavor,
            query,
            limit,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let results =
                app.search_addons(build_search_addons_request(installation, query, limit))?;
            render(json, &results, render_addon_search_catalog)?;
        }
        AddonCommands::List { install, flavor } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let inventory = app.list_addons(build_list_addons_request(installation))?;
            render(json, &inventory, render_addon_inventory)?;
        }
        AddonCommands::Install {
            install,
            flavor,
            source,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let result = app.install_addon(build_install_addon_request(
                installation,
                source,
                dry_run,
                backup_output,
                replace_existing,
            ))?;
            render(json, &result, render_addon_install)?;
        }
        AddonCommands::Update {
            install,
            flavor,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let result = app.update_addons(build_update_addons_request(
                installation,
                name,
                dry_run,
                backup_output,
            ))?;
            render(json, &result, render_addon_update)?;
        }
        AddonCommands::Remove {
            install,
            flavor,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let result = app.remove_addons(build_remove_addons_request(
                installation,
                name,
                dry_run,
                backup_output,
            ))?;
            render(json, &result, render_addon_remove)?;
        }
        AddonCommands::Index { .. } | AddonCommands::Lock { .. } => {
            return Err(AppError::Validation(
                "internal CLI routing error: addon subcommand reached basic addon handler"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
