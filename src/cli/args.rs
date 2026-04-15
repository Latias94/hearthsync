use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::core::install::WowFlavor;

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
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
    },
    Doctor {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },
    Bundle {
        #[command(subcommand)]
        command: BundleCommands,
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
}

#[derive(Debug, Subcommand)]
pub enum AddonCommands {
    #[command(about = "Inspect, install, or update addons from a curated TOML index")]
    Index {
        #[command(subcommand)]
        command: AddonIndexCommands,
    },
    Search {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    List {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
    },
    Install {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(
            long,
            help = "Local zip path, http(s) zip URL, github:owner/repo[@tag][#asset.zip], or curseforge:modId[@fileId] (requires HEARTHSYNC_CURSEFORGE_API_KEY)"
        )]
        source: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
        #[arg(long)]
        replace_existing: bool,
    },
    Update {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
    },
    Remove {
        #[arg(long)]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long)]
        name: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum AddonIndexCommands {
    #[command(about = "Validate and summarize a curated addon index")]
    Inspect {
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
    },
    #[command(about = "Install one package from a curated addon index")]
    Install {
        #[arg(long, help = "World of Warcraft installation or product root")]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
        #[arg(long, help = "Package id or package name from the index")]
        name: String,
        #[arg(long, help = "Preview changes without writing files")]
        dry_run: bool,
        #[arg(long, help = "Directory for automatic backups")]
        backup_output: Option<PathBuf>,
        #[arg(long, help = "Replace existing addon directories instead of failing")]
        replace_existing: bool,
    },
    #[command(about = "Update indexed packages already tracked in the addon registry")]
    Update {
        #[arg(long, help = "World of Warcraft installation or product root")]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
        #[arg(long, help = "Optional package id or package name from the index")]
        name: Option<String>,
        #[arg(long, help = "Preview changes without writing files")]
        dry_run: bool,
        #[arg(long, help = "Directory for automatic backups")]
        backup_output: Option<PathBuf>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum FlavorArg {
    Retail,
    Classic,
    #[value(name = "classic-era")]
    ClassicEra,
    Ptr,
    Beta,
    Xptr,
}

impl From<FlavorArg> for WowFlavor {
    fn from(value: FlavorArg) -> Self {
        match value {
            FlavorArg::Retail => WowFlavor::Retail,
            FlavorArg::Classic => WowFlavor::Classic,
            FlavorArg::ClassicEra => WowFlavor::ClassicEra,
            FlavorArg::Ptr => WowFlavor::Ptr,
            FlavorArg::Beta => WowFlavor::Beta,
            FlavorArg::Xptr => WowFlavor::Xptr,
        }
    }
}
