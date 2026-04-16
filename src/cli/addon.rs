use super::AddonCommands;
use super::addon_index::handle_addon_index_command;
use super::addon_lock::handle_addon_lock_command;
use super::addon_manage::handle_basic_addon_command;
use crate::core::error::AppResult;

pub(super) fn handle_addon_command(json: bool, command: AddonCommands) -> AppResult<()> {
    match command {
        AddonCommands::Index { command } => handle_addon_index_command(json, command)?,
        AddonCommands::Lock { command } => handle_addon_lock_command(json, command)?,
        command => handle_basic_addon_command(json, command)?,
    }

    Ok(())
}
