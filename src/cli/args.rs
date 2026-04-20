use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub(super) mod addon;
pub(super) mod backup;
pub(super) mod bundle;
pub(super) mod external_package;
pub(super) mod shared;

use addon::AddonCommands;
use backup::BackupCommands;
use bundle::BundleCommands;
use external_package::ExternalPackageCommands;
use shared::InstallTargetArgs;

#[derive(Debug, Parser)]
#[command(
    name = "hearthsync",
    version,
    about = "Cross-platform World of Warcraft addon sync tooling"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Scan,
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
    #[command(about = "Analyze, plan, or apply author-provided external UI packages")]
    ExternalPackage {
        #[command(subcommand)]
        command: ExternalPackageCommands,
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
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::shared::FlavorArg;
    use super::{Cli, Commands};

    #[test]
    fn parses_top_level_inspect_with_shared_install_target() {
        let cli = Cli::parse_from([
            "hearthsync",
            "inspect",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
        ]);

        match cli.command {
            Commands::Inspect { install_target } => {
                assert_eq!(
                    install_target.install,
                    PathBuf::from("E:\\Games\\World of Warcraft")
                );
                assert_eq!(install_target.flavor, Some(FlavorArg::Retail));
            }
            _ => panic!("expected inspect command"),
        }
    }
}
