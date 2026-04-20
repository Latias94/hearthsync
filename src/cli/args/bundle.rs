use std::path::PathBuf;

use clap::Subcommand;

use super::FlavorArg;

#[derive(Debug, Subcommand)]
pub enum BundleCommands {
    Pack {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
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
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        mapping_file: Option<PathBuf>,
        #[arg(long)]
        target_account: Option<String>,
        #[arg(long)]
        target_server: Option<String>,
        #[arg(long)]
        target_character: Option<String>,
        #[arg(long = "select-account")]
        selected_accounts: Vec<String>,
        #[arg(long)]
        all_accounts: bool,
    },
    Unpack {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
        #[arg(long)]
        mapping_file: Option<PathBuf>,
        #[arg(long)]
        target_account: Option<String>,
        #[arg(long)]
        target_server: Option<String>,
        #[arg(long)]
        target_character: Option<String>,
        #[arg(long = "select-account")]
        selected_accounts: Vec<String>,
        #[arg(long)]
        all_accounts: bool,
    },
    AddonPlan {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
    },
    AddonApply {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        backup_output: Option<PathBuf>,
        #[arg(long)]
        replace_existing: bool,
    },
}
