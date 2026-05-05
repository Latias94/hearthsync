mod addon;
mod addon_cache;
mod addon_index;
mod addon_lock;
mod addon_manage;
mod addon_policy;
mod app_support;
mod args;
mod backup;
mod bundle;
mod bundle_addon;
mod bundle_apply;
mod bundle_archive;
mod config;
mod external_package;
mod mapping;
mod output;
mod settings;
mod system;
#[cfg(test)]
mod test_support;

use self::app_support::build_runtime;
use clap::Parser;

use crate::core::error::AppResult;

pub(crate) use args::addon::{
    AddonCacheCommands, AddonCommands, AddonIndexCommands, AddonLockCommands, AddonPolicyCommands,
};
pub(crate) use args::backup::BackupCommands;
pub(crate) use args::bundle::BundleCommands;
pub(crate) use args::config::ConfigCommands;
pub(crate) use args::external_package::{
    ExternalPackageBundleOptions, ExternalPackageCommands, ExternalPackageLayoutArg,
    ExternalPackageSourceLayoutArgs, SharingModeArg, WtfScopeArg,
};
pub(crate) use args::settings::SettingsCommands;
pub(crate) use args::shared::{AddonCacheRepairRemotePolicyArg, AddonStateStorageArg};
pub(crate) use args::shared::{
    ApplyMappingArgs, CliRuntimeArgs, InstallTargetArgs, OptionalInstallTargetArgs,
};
#[cfg(test)]
pub(crate) use args::shared::{ApplyPolicyArg, FlavorArg, PlatformArg};
pub(crate) use args::{Cli, Commands, ManifestCommands};

pub fn run() -> AppResult<()> {
    let cli = Cli::parse();
    let runtime = build_runtime(cli.runtime.clone())?;

    match cli.command {
        Commands::Scan => system::handle_scan(cli.json, runtime)?,
        Commands::Runtime { install_target } => {
            system::handle_runtime(cli.json, runtime, install_target)?
        }
        Commands::Inspect { install_target } => {
            system::handle_inspect(cli.json, runtime, install_target)?
        }
        Commands::Doctor { install_target } => {
            system::handle_doctor(cli.json, runtime, install_target)?
        }
        Commands::Backup { command } => backup::handle_backup_command(cli.json, runtime, command)?,
        Commands::Bundle { command } => bundle::handle_bundle_command(cli.json, runtime, command)?,
        Commands::Config { command } => config::handle_config_command(cli.json, runtime, command)?,
        Commands::ExternalPackage { command } => {
            external_package::handle_external_package_command(cli.json, runtime, command)?
        }
        Commands::Settings { command } => {
            settings::handle_settings_command(cli.json, runtime, command)?
        }
        Commands::Addon { command } => addon::handle_addon_command(cli.json, runtime, command)?,
        Commands::Manifest { command } => {
            system::handle_manifest_command(cli.json, runtime, command)?
        }
    }

    Ok(())
}
