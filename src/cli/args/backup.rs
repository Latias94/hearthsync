use std::path::PathBuf;

use clap::Subcommand;

use super::FlavorArg;

#[derive(Debug, Subcommand)]
pub enum BackupCommands {
    Create {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    Restore {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        archive: Option<PathBuf>,
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}
