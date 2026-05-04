use super::AddonCommands;
use super::addon_cache::handle_addon_cache_command;
use super::addon_index::handle_addon_index_command;
use super::addon_lock::handle_addon_lock_command;
use super::addon_manage::{
    handle_addon_adopt, handle_addon_install, handle_addon_list, handle_addon_relink,
    handle_addon_remove, handle_addon_search, handle_addon_update,
};
use super::addon_policy::handle_addon_policy_command;
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

pub(super) fn handle_addon_command(
    json: bool,
    runtime: AppRuntime,
    command: AddonCommands,
) -> AppResult<()> {
    match command {
        AddonCommands::Cache { command } => handle_addon_cache_command(json, runtime, command)?,
        AddonCommands::Index { command } => handle_addon_index_command(json, runtime, command)?,
        AddonCommands::Lock { command } => handle_addon_lock_command(json, runtime, command)?,
        AddonCommands::Policy { command } => handle_addon_policy_command(json, runtime, command)?,
        AddonCommands::Search {
            install_target,
            query,
            limit,
            provider,
        } => handle_addon_search(json, runtime, install_target, query, limit, provider)?,
        AddonCommands::List { install_target } => handle_addon_list(json, runtime, install_target)?,
        AddonCommands::Adopt {
            install_target,
            addon_directories,
            package_id,
            archive_output,
            dry_run,
        } => handle_addon_adopt(
            json,
            runtime,
            install_target,
            addon_directories,
            package_id,
            archive_output,
            dry_run,
        )?,
        AddonCommands::Relink {
            install_target,
            name,
            source,
            dry_run,
        } => handle_addon_relink(json, runtime, install_target, name, source, dry_run)?,
        AddonCommands::Install {
            install_target,
            source,
            dry_run,
            backup_output,
            replace_existing,
        } => handle_addon_install(
            json,
            runtime,
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
        } => handle_addon_update(json, runtime, install_target, name, dry_run, backup_output)?,
        AddonCommands::Remove {
            install_target,
            name,
            dry_run,
            backup_output,
        } => handle_addon_remove(json, runtime, install_target, name, dry_run, backup_output)?,
    }

    Ok(())
}
