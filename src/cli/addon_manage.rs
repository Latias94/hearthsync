use super::AddonCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::output::{
    render, render_addon_install, render_addon_inventory, render_addon_remove,
    render_addon_search_catalog, render_addon_update,
};
use crate::core::app::{
    InstallAddonAppRequest, ListAddonsRequest, RemoveAddonAppRequest, SearchAddonsRequest,
    UpdateAddonAppRequest,
};
use crate::core::error::{AppError, AppResult};

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
            let results = app.search_addons(SearchAddonsRequest {
                installation,
                query,
                limit,
            })?;
            render(json, &results, render_addon_search_catalog)?;
        }
        AddonCommands::List { install, flavor } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let inventory = app.list_addons(ListAddonsRequest { installation })?;
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
            let result = app.install_addon(InstallAddonAppRequest {
                installation,
                source,
                dry_run,
                backup_output_path: backup_output,
                replace_existing,
                metadata: None,
            })?;
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
            let result = app.update_addons(UpdateAddonAppRequest {
                installation,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
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
            let result = app.remove_addons(RemoveAddonAppRequest {
                installation,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
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
