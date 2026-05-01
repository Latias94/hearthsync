use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub(super) mod addon;
pub(super) mod backup;
pub(super) mod bundle;
pub(super) mod config;
pub(super) mod external_package;
pub(super) mod settings;
pub(super) mod shared;

use addon::AddonCommands;
use backup::BackupCommands;
use bundle::BundleCommands;
use config::ConfigCommands;
use external_package::ExternalPackageCommands;
use settings::SettingsCommands;
use shared::{CliRuntimeArgs, InstallTargetArgs, OptionalInstallTargetArgs};

#[derive(Debug, Parser)]
#[command(
    name = "hearthsync",
    version,
    about = "Cross-platform World of Warcraft addon sync tooling"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(flatten)]
    pub runtime: CliRuntimeArgs,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Scan,
    #[command(about = "Show current runtime diagnostics and stable capability projection")]
    Runtime {
        #[command(flatten)]
        install_target: OptionalInstallTargetArgs,
    },
    Inspect {
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    Doctor {
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },
    Bundle {
        #[command(subcommand)]
        command: BundleCommands,
    },
    #[command(about = "Inspect, plan, or apply WoW config packages and UI setup bundles")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    #[command(about = "Analyze, plan, or apply author-provided external UI packages")]
    ExternalPackage {
        #[command(subcommand)]
        command: ExternalPackageCommands,
    },
    #[command(about = "Inspect or persist global HearthSync runtime settings")]
    Settings {
        #[command(subcommand)]
        command: SettingsCommands,
    },
    Addon {
        #[command(subcommand)]
        command: AddonCommands,
    },
    Manifest {
        #[command(subcommand)]
        command: ManifestCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum ManifestCommands {
    Example,
    Validate {
        #[arg(long)]
        file: PathBuf,
    },
}

#[cfg(test)]
mod tests;
