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
#[cfg(test)]
mod test_support;

use clap::Parser;

use crate::core::error::AppResult;

pub(crate) use args::addon::{AddonCommands, AddonIndexCommands, AddonLockCommands};
pub(crate) use args::backup::BackupCommands;
pub(crate) use args::bundle::BundleCommands;
pub(crate) use args::external_package::{ExternalPackageBundleOptions, ExternalPackageCommands};
pub(crate) use args::shared::{ApplyMappingArgs, InstallTargetArgs};
#[cfg(test)]
pub(crate) use args::shared::{ApplyPolicyArg, FlavorArg, PlatformArg};
pub(crate) use args::{Cli, Commands, ManifestCommands};

pub fn run() -> AppResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan => system::handle_scan(cli.json)?,
        Commands::Inspect { install_target } => system::handle_inspect(cli.json, install_target)?,
        Commands::Doctor { install_target } => system::handle_doctor(cli.json, install_target)?,
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
