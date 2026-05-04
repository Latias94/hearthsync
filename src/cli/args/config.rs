use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::external_package::{ExternalPackageBundleOptions, ExternalPackageSourceLayoutArgs};
use super::shared::{ApplyMappingArgs, ApplyPolicyArg, FlavorArg, InstallTargetArgs, PlatformArg};

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    #[command(about = "Inspect a config package and summarize normalized WoW resources")]
    Inspect {
        #[arg(
            long,
            help = "Path to a config package zip file or extracted directory"
        )]
        source: PathBuf,
    },
    #[command(about = "Build a config-sync apply plan without writing files")]
    Plan {
        #[command(flatten)]
        config_options: ConfigPackageOptions,
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[command(flatten)]
        apply_mapping: ApplyMappingArgs,
    },
    #[command(about = "Apply a config package through the shared config-sync pipeline")]
    Apply {
        #[command(flatten)]
        config_options: ConfigPackageOptions,
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        backup_output: Option<PathBuf>,
        #[command(flatten)]
        apply_mapping: ApplyMappingArgs,
    },
}

#[derive(Debug, Clone, Args)]
pub struct ConfigPackageOptions {
    #[arg(
        long,
        help = "Path to a config package zip file or extracted directory"
    )]
    pub source: PathBuf,
    #[arg(long, value_enum, help = "WoW flavor that the config package targets")]
    pub source_flavor: FlavorArg,
    #[arg(long, value_enum, help = "Source platform if known")]
    pub source_platform: Option<PlatformArg>,
    #[arg(
        long = "supported-target",
        value_enum,
        help = "Supported target flavor(s); defaults to source flavor"
    )]
    pub supported_targets: Vec<FlavorArg>,
    #[arg(long, help = "Override the normalized config package id")]
    pub package_id: Option<String>,
    #[arg(long, help = "Override the normalized config package name")]
    pub package_name: Option<String>,
    #[arg(long, help = "Override the manifest created_by field")]
    pub created_by: Option<String>,
    #[arg(long, help = "Override the manifest description")]
    pub description: Option<String>,
    #[arg(
        long,
        help = "Disable backup creation in the generated temporary package manifest"
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

impl From<ConfigPackageOptions> for ExternalPackageBundleOptions {
    fn from(value: ConfigPackageOptions) -> Self {
        Self {
            source: value.source,
            source_layout: ExternalPackageSourceLayoutArgs::default(),
            source_flavor: value.source_flavor,
            source_platform: value.source_platform,
            supported_targets: value.supported_targets,
            package_id: value.package_id,
            package_name: value.package_name,
            created_by: value.created_by,
            description: value.description,
            no_backup: value.no_backup,
            addons_policy: value.addons_policy,
            wtf_common_policy: value.wtf_common_policy,
            wtf_characters_policy: value.wtf_characters_policy,
            fonts_policy: value.fonts_policy,
            interface_assets_policy: value.interface_assets_policy,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::super::shared::{FlavorArg, PlatformArg};
    use super::super::{Cli, Commands};
    use super::ConfigCommands;
    use crate::cli::ApplyPolicyArg;

    #[test]
    fn parses_config_inspect_command() {
        let cli = Cli::parse_from([
            "hearthsync",
            "config",
            "inspect",
            "--source",
            "C:\\temp\\author-ui.zip",
        ]);

        match cli.command {
            Commands::Config { command } => match command {
                ConfigCommands::Inspect { source } => {
                    assert_eq!(source, PathBuf::from("C:\\temp\\author-ui.zip"));
                }
                _ => panic!("expected inspect command"),
            },
            _ => panic!("expected config command"),
        }
    }

    #[test]
    fn parses_config_apply_command_with_mapping_inputs() {
        let cli = Cli::parse_from([
            "hearthsync",
            "config",
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
            Commands::Config { command } => match command {
                ConfigCommands::Apply {
                    config_options,
                    apply_mapping,
                    dry_run,
                    ..
                } => {
                    assert_eq!(config_options.source_flavor, FlavorArg::Retail);
                    assert_eq!(config_options.source_platform, Some(PlatformArg::Windows));
                    assert_eq!(config_options.supported_targets, vec![FlavorArg::Retail]);
                    assert!(dry_run);
                    assert_eq!(apply_mapping.target_account.as_deref(), Some("ACCOUNT"));
                    assert_eq!(apply_mapping.target_server.as_deref(), Some("Illidan"));
                    assert_eq!(
                        apply_mapping.target_character.as_deref(),
                        Some("Examplemage")
                    );
                    assert_eq!(apply_mapping.selected_accounts, vec!["ACCOUNT".to_string()]);
                }
                _ => panic!("expected apply command"),
            },
            _ => panic!("expected config command"),
        }
    }

    #[test]
    fn parses_config_plan_command_with_policy_overrides() {
        let cli = Cli::parse_from([
            "hearthsync",
            "config",
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
            Commands::Config { command } => match command {
                ConfigCommands::Plan { config_options, .. } => {
                    assert_eq!(config_options.package_id.as_deref(), Some("author-ui"));
                    assert_eq!(config_options.package_name.as_deref(), Some("Author UI"));
                    assert_eq!(
                        config_options.created_by.as_deref(),
                        Some("newbeebox-import")
                    );
                    assert_eq!(
                        config_options.description.as_deref(),
                        Some("normalized import")
                    );
                    assert!(config_options.no_backup);
                    assert_eq!(config_options.addons_policy, Some(ApplyPolicyArg::Mirror));
                    assert_eq!(
                        config_options.wtf_common_policy,
                        Some(ApplyPolicyArg::Share)
                    );
                }
                _ => panic!("expected plan command"),
            },
            _ => panic!("expected config command"),
        }
    }
}
