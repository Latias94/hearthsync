use std::path::PathBuf;

use clap::Subcommand;

use super::shared::{InstallTargetArgs, ReleaseChannelArg};

#[derive(Debug, Subcommand)]
pub enum AddonCommands {
    #[command(about = "Inspect or repair provider download cache state")]
    Cache {
        #[command(subcommand)]
        command: AddonCacheCommands,
    },
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
    #[command(about = "Inspect or manage persisted addon update preferences")]
    Policy {
        #[command(subcommand)]
        command: AddonPolicyCommands,
    },
    Search {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
        #[arg(
            long,
            help = "Restrict catalog search to one provider id, for example curseforge"
        )]
        provider: Option<String>,
    },
    List {
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    Adopt {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(
            long = "addon",
            required = true,
            value_name = "DIRECTORY",
            help = "Explicit untracked addon directory to adopt; repeat for multi-addon packages"
        )]
        addon_directories: Vec<String>,
        #[arg(
            long,
            help = "Optional tracked package id; required when adopting multiple addon directories into one package"
        )]
        package_id: Option<String>,
        #[arg(
            long,
            help = "Optional snapshot archive output path; defaults to the selected installation's configured managed addon-state backend adopted archive path"
        )]
        archive_output: Option<PathBuf>,
        #[arg(
            long,
            help = "Preview the tracked package and snapshot path without writing files"
        )]
        dry_run: bool,
    },
    Relink {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Tracked package id or tracked addon directory to relink")]
        name: String,
        #[arg(
            long,
            help = "Local zip path, http(s) zip URL, github:owner/repo[@tag][#asset.zip], or curseforge:modId[@fileId] (requires HEARTHSYNC_CURSEFORGE_API_KEY or CURSEFORGE_API_KEY)"
        )]
        source: String,
        #[arg(long, help = "Preview the source relink without writing the registry")]
        dry_run: bool,
    },
    Install {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(
            long,
            help = "Local zip path, http(s) zip URL, github:owner/repo[@tag][#asset.zip], or curseforge:modId[@fileId] (requires HEARTHSYNC_CURSEFORGE_API_KEY or CURSEFORGE_API_KEY)"
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
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
    },
    Remove {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        name: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub enum AddonCacheCommands {
    #[command(
        about = "Delete all cached downloaded addon archives from the configured cache directory"
    )]
    Purge,
    #[command(about = "Delete incomplete, orphaned, or invalid cached downloaded addon archives")]
    Repair,
}

#[derive(Debug, Subcommand)]
pub enum AddonPolicyCommands {
    #[command(about = "Read the current addon policy file")]
    Inspect {
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    #[command(about = "Merge one addon policy entry into the current policy file")]
    Set {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Tracked package id or tracked addon directory name")]
        package: String,
        #[arg(long, help = "Explicit ignore override for update planning")]
        ignored: Option<bool>,
        #[arg(long, help = "Pin the addon to a specific version string")]
        pinned_version: Option<String>,
        #[arg(long, help = "Pin the addon to a specific provider file id")]
        pinned_file_id: Option<u32>,
        #[arg(
            long,
            value_enum,
            help = "Preferred release channel for future updates"
        )]
        release_channel: Option<ReleaseChannelArg>,
        #[arg(long, help = "Allow prerelease builds for this addon")]
        allow_prerelease: Option<bool>,
        #[arg(long, help = "Install dependencies for this addon when supported")]
        install_dependencies: Option<bool>,
    },
    #[command(about = "Remove one addon policy entry")]
    Remove {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Tracked package id or existing addon policy package id")]
        package: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum AddonIndexCommands {
    #[command(about = "Inspect and summarize a curated addon index")]
    Inspect {
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
    },
    #[command(about = "Validate a curated addon index and fail on curator warnings")]
    Validate {
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
    },
    #[command(about = "Scaffold a curated addon index from the current tracked addon registry")]
    Scaffold {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Destination addon index TOML file")]
        file: PathBuf,
        #[arg(long, help = "Human-readable name for the generated index")]
        index_name: String,
        #[arg(long, help = "Optional description for the generated index")]
        description: Option<String>,
        #[arg(
            long,
            help = "Optional tracked package id or tracked addon directory name"
        )]
        name: Option<String>,
        #[arg(long, help = "Overwrite an existing addon index file")]
        overwrite: bool,
    },
    #[command(about = "Suggest exact identity hints from the current tracked addon registry")]
    Suggest {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
        #[arg(long, help = "Optional package id or package name from the index")]
        name: Option<String>,
    },
    #[command(
        about = "Bulk attach tracked packages to curated addon index packages without reinstalling files"
    )]
    Attach {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
        #[arg(long, help = "Optional package id or package name from the index")]
        name: Option<String>,
        #[arg(
            long,
            help = "Preview the bulk attach plan without writing the registry"
        )]
        dry_run: bool,
        #[arg(
            long,
            help = "Apply ready packages even when other selected packages are blocked"
        )]
        apply_ready_only: bool,
    },
    #[command(about = "Install one package from a curated addon index")]
    Install {
        #[command(flatten)]
        install_target: InstallTargetArgs,
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
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
        #[arg(long, help = "Optional package id or package name from the index")]
        name: Option<String>,
        #[arg(long, help = "Preview changes without writing files")]
        dry_run: bool,
        #[arg(long, help = "Directory for automatic backups")]
        backup_output: Option<PathBuf>,
    },
    #[command(about = "Relink one tracked package to a curated addon index package")]
    Relink {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long, help = "Path to the addon index TOML file")]
        file: PathBuf,
        #[arg(long, help = "Package id or package name from the index")]
        name: String,
        #[arg(
            long,
            help = "Optional tracked package id or tracked addon directory name; otherwise HearthSync auto-matches one tracked package"
        )]
        target: Option<String>,
        #[arg(long, help = "Preview the relink without writing the registry")]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum AddonLockCommands {
    #[command(about = "Read the current addon lock file")]
    Inspect {
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    #[command(about = "Regenerate the addon lock file from the addon registry")]
    Write {
        #[command(flatten)]
        install_target: InstallTargetArgs,
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
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(
            long,
            help = "Optional addon lock TOML file; defaults to the selected installation's configured managed addon-state backend addon lock"
        )]
        file: Option<PathBuf>,
    },
    #[command(about = "Build a sync plan from an addon lock file")]
    Plan {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(
            long,
            help = "Optional addon lock TOML file; defaults to the selected installation's configured managed addon-state backend addon lock"
        )]
        file: Option<PathBuf>,
    },
    #[command(about = "Apply an addon lock sync plan to the current installation")]
    Apply {
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(
            long,
            help = "Optional addon lock TOML file; defaults to the selected installation's configured managed addon-state backend addon lock"
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
