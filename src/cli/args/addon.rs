use std::path::PathBuf;

use clap::Subcommand;

use super::FlavorArg;

#[derive(Debug, Subcommand)]
pub enum AddonCommands {
    #[command(about = "Inspect, install, or update addons from a curated TOML index")]
    Index {
        #[command(subcommand)]
        command: AddonIndexCommands,
    },
    #[command(about = "Inspect, compare, plan, or apply addon locks")]
    Lock {
        #[command(subcommand)]
        command: AddonLockCommands,
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
pub enum AddonLockCommands {
    #[command(about = "Read the current addon lock file")]
    Inspect {
        #[arg(long, help = "World of Warcraft installation or product root")]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
    },
    #[command(about = "Regenerate the addon lock file from the addon registry")]
    Write {
        #[arg(long, help = "World of Warcraft installation or product root")]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
    },
    #[command(about = "Compare two addon lock files")]
    Diff {
        #[arg(long, help = "Left addon lock TOML file")]
        left_file: PathBuf,
        #[arg(long, help = "Right addon lock TOML file")]
        right_file: PathBuf,
    },
    #[command(about = "Verify the current installation against an addon lock file")]
    Verify {
        #[arg(long, help = "World of Warcraft installation or product root")]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(
            long,
            help = "Optional addon lock TOML file; defaults to Interface/AddOns/.hearthsync/lock.toml"
        )]
        file: Option<PathBuf>,
    },
    #[command(about = "Build a sync plan from an addon lock file")]
    Plan {
        #[arg(long, help = "World of Warcraft installation or product root")]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(
            long,
            help = "Optional addon lock TOML file; defaults to Interface/AddOns/.hearthsync/lock.toml"
        )]
        file: Option<PathBuf>,
    },
    #[command(about = "Apply an addon lock sync plan to the current installation")]
    Apply {
        #[arg(long, help = "World of Warcraft installation or product root")]
        install: PathBuf,
        #[arg(long, value_enum)]
        flavor: Option<FlavorArg>,
        #[arg(
            long,
            help = "Optional addon lock TOML file; defaults to Interface/AddOns/.hearthsync/lock.toml"
        )]
        file: Option<PathBuf>,
        #[arg(long, help = "Directory for automatic backups")]
        backup_output: Option<PathBuf>,
        #[arg(
            long,
            help = "Allow overwriting conflicting untracked addon directories"
        )]
        replace_existing: bool,
    },
}
