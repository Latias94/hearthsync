use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod addon;
mod backup;
mod bundle;
mod external_package;
mod shared;

pub use addon::{AddonCommands, AddonIndexCommands, AddonLockCommands};
pub use backup::BackupCommands;
pub use bundle::BundleCommands;
pub use external_package::{ExternalPackageBundleOptions, ExternalPackageCommands};
pub use shared::{ApplyMappingArgs, ApplyPolicyArg, FlavorArg, InstallTargetArgs, PlatformArg};

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
    use clap::Parser;

    use super::*;

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
