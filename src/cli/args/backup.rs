use std::path::PathBuf;

use clap::Subcommand;

use super::InstallTargetArgs;

#[derive(Debug, Subcommand)]
pub enum BackupCommands {
    Create {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    Restore {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        archive: Option<PathBuf>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}
