use super::AddonCommands;
use super::addon_index::handle_addon_index_command;
use super::addon_lock::handle_addon_lock_command;
use super::addon_manage::{
    handle_addon_install, handle_addon_list, handle_addon_remove, handle_addon_search,
    handle_addon_update,
};
use crate::core::error::AppResult;

pub(super) fn handle_addon_command(json: bool, command: AddonCommands) -> AppResult<()> {
    match command {
        AddonCommands::Index { command } => handle_addon_index_command(json, command)?,
        AddonCommands::Lock { command } => handle_addon_lock_command(json, command)?,
        AddonCommands::Search {
            install_target,
            query,
            limit,
        } => handle_addon_search(json, install_target, query, limit)?,
        AddonCommands::List { install_target } => handle_addon_list(json, install_target)?,
        AddonCommands::Install {
            install_target,
            source,
            dry_run,
            backup_output,
            replace_existing,
        } => handle_addon_install(
            json,
            install_target,
            source,
            dry_run,
            backup_output,
            replace_existing,
        )?,
        AddonCommands::Update {
            install_target,
            name,
            dry_run,
            backup_output,
        } => handle_addon_update(json, install_target, name, dry_run, backup_output)?,
        AddonCommands::Remove {
            install_target,
            name,
            dry_run,
            backup_output,
        } => handle_addon_remove(json, install_target, name, dry_run, backup_output)?,
    }

    Ok(())
}
