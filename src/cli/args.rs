use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::manifest::ResourceApplyPolicy;

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

#[derive(Debug, Subcommand)]
pub enum ExternalPackageCommands {
    #[command(about = "Analyze an external UI package and summarize normalized resources")]
    Inspect {
        #[arg(
            long,
            help = "Path to an author-provided zip file or extracted directory"
        )]
        source: PathBuf,
    },
    #[command(about = "Build an apply plan for an external UI package without writing files")]
    Plan {
        #[command(flatten)]
        bundle_options: ExternalPackageBundleOptions,
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
    #[command(
        about = "Apply an external UI package directly through the normalized bundle pipeline"
    )]
    Apply {
        #[command(flatten)]
        bundle_options: ExternalPackageBundleOptions,
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

#[derive(Debug, Clone, Args)]
pub struct ExternalPackageBundleOptions {
    #[arg(
        long,
        help = "Path to an author-provided zip file or extracted directory"
    )]
    pub source: PathBuf,
    #[arg(long, value_enum, help = "WoW flavor that the source package targets")]
    pub source_flavor: FlavorArg,
    #[arg(long, value_enum, help = "Source platform if known")]
    pub source_platform: Option<PlatformArg>,
    #[arg(
        long = "supported-target",
        value_enum,
        help = "Supported target flavor(s); defaults to source flavor"
    )]
    pub supported_targets: Vec<FlavorArg>,
    #[arg(long, help = "Override the normalized package id")]
    pub package_id: Option<String>,
    #[arg(long, help = "Override the normalized package name")]
    pub package_name: Option<String>,
    #[arg(long, help = "Override the manifest created_by field")]
    pub created_by: Option<String>,
    #[arg(long, help = "Override the manifest description")]
    pub description: Option<String>,
    #[arg(
        long,
        help = "Disable backup creation in the generated temporary bundle manifest"
    )]
    pub no_backup: bool,
    #[arg(long, value_enum, help = "Override addon apply policy")]
    pub addons_policy: Option<ApplyPolicyArg>,
    #[arg(long, value_enum, help = "Override common WTF apply policy")]
    pub wtf_common_policy: Option<ApplyPolicyArg>,
    #[arg(long, value_enum, help = "Override character WTF apply policy")]
    pub wtf_characters_policy: Option<ApplyPolicyArg>,
    #[arg(long, value_enum, help = "Override fonts apply policy")]
    pub fonts_policy: Option<ApplyPolicyArg>,
    #[arg(long, value_enum, help = "Override interface assets apply policy")]
    pub interface_assets_policy: Option<ApplyPolicyArg>,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PlatformArg {
    Windows,
    #[value(name = "macos")]
    MacOs,
    Linux,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ApplyPolicyArg {
    Merge,
    Share,
    Sync,
    Mirror,
    #[value(name = "replace-selected")]
    ReplaceSelected,
    Preserve,
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

impl From<PlatformArg> for HostPlatform {
    fn from(value: PlatformArg) -> Self {
        match value {
            PlatformArg::Windows => HostPlatform::Windows,
            PlatformArg::MacOs => HostPlatform::MacOs,
            PlatformArg::Linux => HostPlatform::Linux,
            PlatformArg::Unknown => HostPlatform::Unknown,
        }
    }
}

impl From<ApplyPolicyArg> for ResourceApplyPolicy {
    fn from(value: ApplyPolicyArg) -> Self {
        match value {
            ApplyPolicyArg::Merge => ResourceApplyPolicy::Merge,
            ApplyPolicyArg::Share => ResourceApplyPolicy::Share,
            ApplyPolicyArg::Sync => ResourceApplyPolicy::Sync,
            ApplyPolicyArg::Mirror => ResourceApplyPolicy::Mirror,
            ApplyPolicyArg::ReplaceSelected => ResourceApplyPolicy::ReplaceSelected,
            ApplyPolicyArg::Preserve => ResourceApplyPolicy::Preserve,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn parses_external_package_inspect_command() {
        let cli = Cli::parse_from([
            "hearthsync",
            "external-package",
            "inspect",
            "--source",
            "C:\\temp\\author-ui.zip",
        ]);

        match cli.command {
            Commands::ExternalPackage { command } => match command {
                ExternalPackageCommands::Inspect { source } => {
                    assert_eq!(source, PathBuf::from("C:\\temp\\author-ui.zip"));
                }
                _ => panic!("expected inspect command"),
            },
            _ => panic!("expected external-package command"),
        }
    }

    #[test]
    fn parses_external_package_apply_command_with_mapping_inputs() {
        let cli = Cli::parse_from([
            "hearthsync",
            "external-package",
            "apply",
            "--source",
            "C:\\temp\\author-ui.zip",
            "--source-flavor",
            "retail",
            "--source-platform",
            "windows",
            "--supported-target",
            "retail",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--dry-run",
            "--target-account",
            "ACCOUNT",
            "--target-server",
            "Illidan",
            "--target-character",
            "Examplemage",
            "--select-account",
            "ACCOUNT",
        ]);

        match cli.command {
            Commands::ExternalPackage { command } => match command {
                ExternalPackageCommands::Apply {
                    bundle_options,
                    dry_run,
                    target_account,
                    target_server,
                    target_character,
                    selected_accounts,
                    ..
                } => {
                    assert_eq!(bundle_options.source_flavor, FlavorArg::Retail);
                    assert_eq!(bundle_options.source_platform, Some(PlatformArg::Windows));
                    assert_eq!(bundle_options.supported_targets, vec![FlavorArg::Retail]);
                    assert!(dry_run);
                    assert_eq!(target_account.as_deref(), Some("ACCOUNT"));
                    assert_eq!(target_server.as_deref(), Some("Illidan"));
                    assert_eq!(target_character.as_deref(), Some("Examplemage"));
                    assert_eq!(selected_accounts, vec!["ACCOUNT".to_string()]);
                }
                _ => panic!("expected apply command"),
            },
            _ => panic!("expected external-package command"),
        }
    }

    #[test]
    fn parses_external_package_plan_command_with_bundle_overrides() {
        let cli = Cli::parse_from([
            "hearthsync",
            "external-package",
            "plan",
            "--source",
            "C:\\temp\\author-ui.zip",
            "--source-flavor",
            "retail",
            "--package-id",
            "author-ui",
            "--package-name",
            "Author UI",
            "--created-by",
            "newbeebox-import",
            "--description",
            "normalized import",
            "--no-backup",
            "--addons-policy",
            "mirror",
            "--wtf-common-policy",
            "share",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
        ]);

        match cli.command {
            Commands::ExternalPackage { command } => match command {
                ExternalPackageCommands::Plan { bundle_options, .. } => {
                    assert_eq!(bundle_options.package_id.as_deref(), Some("author-ui"));
                    assert_eq!(bundle_options.package_name.as_deref(), Some("Author UI"));
                    assert_eq!(
                        bundle_options.created_by.as_deref(),
                        Some("newbeebox-import")
                    );
                    assert_eq!(
                        bundle_options.description.as_deref(),
                        Some("normalized import")
                    );
                    assert!(bundle_options.no_backup);
                    assert_eq!(bundle_options.addons_policy, Some(ApplyPolicyArg::Mirror));
                    assert_eq!(
                        bundle_options.wtf_common_policy,
                        Some(ApplyPolicyArg::Share)
                    );
                }
                _ => panic!("expected plan command"),
            },
            _ => panic!("expected external-package command"),
        }
    }
}
