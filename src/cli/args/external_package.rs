use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use super::shared::{ApplyMappingArgs, ApplyPolicyArg, FlavorArg, InstallTargetArgs, PlatformArg};

#[derive(Debug, Subcommand)]
pub enum ExternalPackageCommands {
    #[command(about = "Analyze an external UI package and summarize normalized resources")]
    Inspect {
        #[arg(
            long,
            help = "Path to an author-provided zip file or extracted directory"
        )]
        source: PathBuf,
        #[command(flatten)]
        source_layout: ExternalPackageSourceLayoutArgs,
    },
    #[command(about = "Create a reusable HearthSync bundle from an external UI package")]
    Bundle {
        #[command(flatten)]
        bundle_options: ExternalPackageBundleOptions,
        #[arg(long, help = "Output bundle zip path")]
        output: PathBuf,
        #[arg(
            long,
            value_enum,
            default_value = "private",
            help = "Sharing policy mode for export review"
        )]
        sharing_mode: SharingModeArg,
        #[arg(
            long,
            help = "Allow public export even when sharing review reports required risks"
        )]
        allow_public_sharing_risks: bool,
        #[arg(
            long = "exclude-wtf-scope",
            value_enum,
            help = "Exclude normalized WTF entries with this scope from the exported bundle"
        )]
        excluded_wtf_scopes: Vec<WtfScopeArg>,
    },
    #[command(about = "Build an apply plan for an external UI package without writing files")]
    Plan {
        #[command(flatten)]
        bundle_options: ExternalPackageBundleOptions,
        #[command(flatten)]
        install_target: InstallTargetArgs,
        #[command(flatten)]
        apply_mapping: ApplyMappingArgs,
    },
    #[command(
        about = "Apply an external UI package directly through the normalized bundle pipeline"
    )]
    Apply {
        #[command(flatten)]
        bundle_options: ExternalPackageBundleOptions,
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
pub struct ExternalPackageBundleOptions {
    #[arg(
        long,
        help = "Path to an author-provided zip file or extracted directory"
    )]
    pub source: PathBuf,
    #[command(flatten)]
    pub source_layout: ExternalPackageSourceLayoutArgs,
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

#[derive(Debug, Clone, Default, Args)]
pub struct ExternalPackageSourceLayoutArgs {
    #[arg(long, value_enum, help = "External package source layout")]
    pub layout: Option<ExternalPackageLayoutArg>,
    #[arg(long, help = "Source account for flat NewBeeBox WTF packages")]
    pub source_account: Option<String>,
    #[arg(long, help = "Source server for flat NewBeeBox character WTF packages")]
    pub source_server: Option<String>,
    #[arg(
        long,
        help = "Source character override for flat NewBeeBox character WTF packages"
    )]
    pub source_character: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ExternalPackageLayoutArg {
    #[default]
    Auto,
    Generic,
    #[value(name = "newbeebox-addon")]
    NewBeeBoxAddon,
    #[value(name = "newbeebox-font")]
    NewBeeBoxFont,
    #[value(name = "newbeebox-material")]
    NewBeeBoxMaterial,
    #[value(name = "newbeebox-wtf-account")]
    NewBeeBoxWtfAccount,
    #[value(name = "newbeebox-wtf-character")]
    NewBeeBoxWtfCharacter,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum SharingModeArg {
    #[default]
    Private,
    Public,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum WtfScopeArg {
    GlobalConfig,
    RootSavedVariables,
    AccountRootFile,
    AccountSavedVariables,
    CharacterSavedVariables,
    CharacterState,
    CacheLike,
    Unknown,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::super::shared::{FlavorArg, PlatformArg};
    use super::super::{Cli, Commands};
    use super::{ExternalPackageCommands, ExternalPackageLayoutArg, SharingModeArg, WtfScopeArg};
    use crate::cli::ApplyPolicyArg;

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
                ExternalPackageCommands::Inspect {
                    source,
                    source_layout,
                } => {
                    assert_eq!(source, PathBuf::from("C:\\temp\\author-ui.zip"));
                    assert_eq!(source_layout.layout, None);
                }
                _ => panic!("expected inspect command"),
            },
            _ => panic!("expected external-package command"),
        }
    }

    #[test]
    fn parses_external_package_inspect_with_newbeebox_layout_context() {
        let cli = Cli::parse_from([
            "hearthsync",
            "external-package",
            "inspect",
            "--source",
            "C:\\temp\\wtfrole-example.zip",
            "--layout",
            "newbeebox-wtf-character",
            "--source-account",
            "ACCOUNT",
            "--source-server",
            "Illidan",
            "--source-character",
            "Sourcechar",
        ]);

        match cli.command {
            Commands::ExternalPackage { command } => match command {
                ExternalPackageCommands::Inspect {
                    source,
                    source_layout,
                } => {
                    assert_eq!(source, PathBuf::from("C:\\temp\\wtfrole-example.zip"));
                    assert_eq!(
                        source_layout.layout,
                        Some(ExternalPackageLayoutArg::NewBeeBoxWtfCharacter)
                    );
                    assert_eq!(source_layout.source_account.as_deref(), Some("ACCOUNT"));
                    assert_eq!(source_layout.source_server.as_deref(), Some("Illidan"));
                    assert_eq!(
                        source_layout.source_character.as_deref(),
                        Some("Sourcechar")
                    );
                }
                _ => panic!("expected inspect command"),
            },
            _ => panic!("expected external-package command"),
        }
    }

    #[test]
    fn parses_external_package_bundle_command_with_output() {
        let cli = Cli::parse_from([
            "hearthsync",
            "external-package",
            "bundle",
            "--source",
            "C:\\temp\\author-ui.zip",
            "--source-flavor",
            "retail",
            "--output",
            "C:\\temp\\author-ui.hearthsync.zip",
            "--sharing-mode",
            "public",
            "--allow-public-sharing-risks",
            "--exclude-wtf-scope",
            "account-saved-variables",
        ]);

        match cli.command {
            Commands::ExternalPackage { command } => match command {
                ExternalPackageCommands::Bundle {
                    bundle_options,
                    output,
                    sharing_mode,
                    allow_public_sharing_risks,
                    excluded_wtf_scopes,
                } => {
                    assert_eq!(
                        bundle_options.source,
                        PathBuf::from("C:\\temp\\author-ui.zip")
                    );
                    assert_eq!(bundle_options.source_flavor, FlavorArg::Retail);
                    assert_eq!(output, PathBuf::from("C:\\temp\\author-ui.hearthsync.zip"));
                    assert_eq!(sharing_mode, SharingModeArg::Public);
                    assert!(allow_public_sharing_risks);
                    assert_eq!(
                        excluded_wtf_scopes,
                        vec![WtfScopeArg::AccountSavedVariables]
                    );
                }
                _ => panic!("expected bundle command"),
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
                    apply_mapping,
                    dry_run,
                    ..
                } => {
                    assert_eq!(bundle_options.source_flavor, FlavorArg::Retail);
                    assert_eq!(bundle_options.source_platform, Some(PlatformArg::Windows));
                    assert_eq!(bundle_options.supported_targets, vec![FlavorArg::Retail]);
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
