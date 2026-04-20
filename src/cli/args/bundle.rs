use std::path::PathBuf;

use clap::Subcommand;

use super::{ApplyMappingArgs, InstallTargetArgs};

#[derive(Debug, Subcommand)]
pub enum BundleCommands {
    Pack {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    Inspect {
        #[arg(long)]
        bundle: PathBuf,
    },
    Plan {
        #[arg(long)]
        bundle: PathBuf,
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[command(flatten)]
        apply_mapping: ApplyMappingArgs,
    },
    Unpack {
        #[arg(long)]
        bundle: PathBuf,
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
        #[command(flatten)]
        apply_mapping: ApplyMappingArgs,
    },
    AddonPlan {
        #[arg(long)]
        bundle: PathBuf,
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    AddonApply {
        #[arg(long)]
        bundle: PathBuf,
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        backup_output: Option<PathBuf>,
        #[arg(long)]
        replace_existing: bool,
    },
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::super::{Cli, Commands};
    use super::*;

    #[test]
    fn parses_bundle_unpack_with_shared_install_and_mapping_args() {
        let cli = Cli::parse_from([
            "hearthsync",
            "bundle",
            "unpack",
            "--bundle",
            "E:\\exports\\ui.bundle.zip",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--dry-run",
            "--mapping-file",
            "E:\\exports\\mapping.toml",
            "--target-account",
            "ACCOUNT",
            "--target-server",
            "Illidan",
            "--target-character",
            "Examplemage",
            "--select-account",
            "ACCOUNT",
            "--all-accounts",
        ]);

        match cli.command {
            Commands::Bundle { command } => match command {
                BundleCommands::Unpack {
                    bundle,
                    install_target,
                    apply_mapping,
                    dry_run,
                    ..
                } => {
                    assert_eq!(bundle, PathBuf::from("E:\\exports\\ui.bundle.zip"));
                    assert_eq!(
                        install_target.install,
                        PathBuf::from("E:\\Games\\World of Warcraft")
                    );
                    assert_eq!(install_target.flavor, Some(super::super::FlavorArg::Retail));
                    assert!(dry_run);
                    assert_eq!(
                        apply_mapping.mapping_file,
                        Some(PathBuf::from("E:\\exports\\mapping.toml"))
                    );
                    assert_eq!(apply_mapping.target_account.as_deref(), Some("ACCOUNT"));
                    assert_eq!(apply_mapping.target_server.as_deref(), Some("Illidan"));
                    assert_eq!(
                        apply_mapping.target_character.as_deref(),
                        Some("Examplemage")
                    );
                    assert_eq!(apply_mapping.selected_accounts, vec!["ACCOUNT".to_string()]);
                    assert!(apply_mapping.all_accounts);
                }
                _ => panic!("expected unpack command"),
            },
            _ => panic!("expected bundle command"),
        }
    }
}
