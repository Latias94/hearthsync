mod addon;
mod addon_index;
mod addon_lock;
mod addon_manage;
mod app_support;
mod args;
mod backup;
mod bundle;
mod bundle_addon;
mod bundle_apply;
mod bundle_archive;
mod external_package;
mod mapping;
mod output;
mod system;

use clap::Parser;

use crate::core::error::AppResult;

pub use args::*;

pub fn run() -> AppResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan => system::handle_scan(cli.json)?,
        Commands::Inspect { install, flavor } => system::handle_inspect(cli.json, install, flavor)?,
        Commands::Doctor { install, flavor } => system::handle_doctor(cli.json, install, flavor)?,
        Commands::Backup { command } => backup::handle_backup_command(cli.json, command)?,
        Commands::Bundle { command } => bundle::handle_bundle_command(cli.json, command)?,
        Commands::ExternalPackage { command } => {
            external_package::handle_external_package_command(cli.json, command)?
        }
        Commands::Addon { command } => addon::handle_addon_command(cli.json, command)?,
        Commands::Manifest { command } => system::handle_manifest_command(cli.json, command)?,
    }

    Ok(())
}
