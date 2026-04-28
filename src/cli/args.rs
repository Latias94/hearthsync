use std::path::PathBuf;

use clap::{Parser, Subcommand};

pub(super) mod addon;
pub(super) mod backup;
pub(super) mod bundle;
pub(super) mod config;
pub(super) mod external_package;
pub(super) mod settings;
pub(super) mod shared;

use addon::AddonCommands;
use backup::BackupCommands;
use bundle::BundleCommands;
use config::ConfigCommands;
use external_package::ExternalPackageCommands;
use settings::SettingsCommands;
use shared::{CliRuntimeArgs, InstallTargetArgs, OptionalInstallTargetArgs};

#[derive(Debug, Parser)]
#[command(
    name = "hearthsync",
    version,
    about = "Cross-platform World of Warcraft addon sync tooling"
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(flatten)]
    pub runtime: CliRuntimeArgs,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Scan,
    #[command(about = "Show current runtime diagnostics and stable capability projection")]
    Runtime {
        #[command(flatten)]
        install_target: OptionalInstallTargetArgs,
    },
    Inspect {
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    Doctor {
        #[command(flatten)]
        install_target: InstallTargetArgs,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommands,
    },
    Bundle {
        #[command(subcommand)]
        command: BundleCommands,
    },
    #[command(about = "Inspect, plan, or apply WoW config packages and UI setup bundles")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    #[command(about = "Analyze, plan, or apply author-provided external UI packages")]
    ExternalPackage {
        #[command(subcommand)]
        command: ExternalPackageCommands,
    },
    #[command(about = "Inspect or persist global HearthSync runtime settings")]
    Settings {
        #[command(subcommand)]
        command: SettingsCommands,
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
pub enum ManifestCommands {
    Example,
    Validate {
        #[arg(long)]
        file: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::addon::{AddonCacheCommands, AddonIndexCommands, AddonPolicyCommands};
    use super::config::ConfigCommands;
    use super::settings::SettingsCommands;
    use super::shared::{AddonStateStorageArg, FlavorArg, ReleaseChannelArg};
    use super::{AddonCommands, Cli, Commands};

    #[test]
    fn parses_top_level_inspect_with_shared_install_target() {
        let cli = Cli::parse_from([
            "hearthsync",
            "inspect",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
        ]);

        match cli.command {
            Commands::Inspect { install_target } => {
                assert_eq!(
                    install_target.install,
                    PathBuf::from("E:\\Games\\World of Warcraft")
                );
                assert_eq!(install_target.flavor, Some(FlavorArg::Retail));
            }
            _ => panic!("expected inspect command"),
        }
    }

    #[test]
    fn parses_top_level_runtime_command() {
        let cli = Cli::parse_from(["hearthsync", "runtime"]);

        match cli.command {
            Commands::Runtime { install_target } => {
                assert_eq!(install_target.install, None);
                assert_eq!(install_target.flavor, None);
            }
            _ => panic!("expected runtime command"),
        }
    }

    #[test]
    fn parses_top_level_runtime_command_with_install_target() {
        let cli = Cli::parse_from([
            "hearthsync",
            "runtime",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
        ]);

        match cli.command {
            Commands::Runtime { install_target } => {
                assert_eq!(
                    install_target.install,
                    Some(PathBuf::from("E:\\Games\\World of Warcraft"))
                );
                assert_eq!(install_target.flavor, Some(FlavorArg::Retail));
            }
            _ => panic!("expected runtime command"),
        }
    }

    #[test]
    fn parses_global_addon_cache_runtime_options() {
        let cli = Cli::parse_from([
            "hearthsync",
            "--addon-cache-dir",
            "E:\\Cache",
            "--addon-http-no-validator-window-secs",
            "120",
            "runtime",
        ]);

        assert_eq!(
            cli.runtime.addon_cache_dir,
            Some(PathBuf::from("E:\\Cache"))
        );
        assert_eq!(cli.runtime.addon_http_no_validator_window_secs, Some(120));
        assert!(!cli.runtime.addon_http_no_validator_always_refresh);
    }

    #[test]
    fn parses_global_addon_cache_always_refresh_option() {
        let cli = Cli::parse_from([
            "hearthsync",
            "--addon-http-no-validator-always-refresh",
            "addon",
            "cache",
            "repair",
        ]);

        assert!(cli.runtime.addon_http_no_validator_always_refresh);
        assert_eq!(cli.runtime.addon_http_no_validator_window_secs, None);
    }

    #[test]
    fn parses_settings_set_command() {
        let cli = Cli::parse_from([
            "hearthsync",
            "settings",
            "set",
            "--addon-state-storage",
            "sidecar",
            "--addon-cache-dir",
            "E:\\Cache",
            "--addon-http-no-validator-window-secs",
            "120",
        ]);

        match cli.command {
            Commands::Settings { command } => match command {
                SettingsCommands::Set {
                    addon_state_storage,
                    addon_cache_dir,
                    addon_http_no_validator_always_refresh,
                    addon_http_no_validator_window_secs,
                    ..
                } => {
                    assert_eq!(addon_state_storage, Some(AddonStateStorageArg::Sidecar));
                    assert_eq!(addon_cache_dir, Some(PathBuf::from("E:\\Cache")));
                    assert!(!addon_http_no_validator_always_refresh);
                    assert_eq!(addon_http_no_validator_window_secs, Some(120));
                }
                _ => panic!("expected settings set command"),
            },
            _ => panic!("expected settings command"),
        }
    }

    #[test]
    fn parses_settings_reset_command() {
        let cli = Cli::parse_from(["hearthsync", "settings", "reset"]);

        match cli.command {
            Commands::Settings { command } => match command {
                SettingsCommands::Reset => {}
                _ => panic!("expected settings reset command"),
            },
            _ => panic!("expected settings command"),
        }
    }

    #[test]
    fn parses_top_level_config_plan_with_install_target() {
        let cli = Cli::parse_from([
            "hearthsync",
            "config",
            "plan",
            "--source",
            "C:\\temp\\author-ui.zip",
            "--source-flavor",
            "retail",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
        ]);

        match cli.command {
            Commands::Config { command } => match command {
                ConfigCommands::Plan {
                    config_options,
                    install_target,
                    ..
                } => {
                    assert_eq!(
                        config_options.source,
                        PathBuf::from("C:\\temp\\author-ui.zip")
                    );
                    assert_eq!(config_options.source_flavor, FlavorArg::Retail);
                    assert_eq!(
                        install_target.install,
                        PathBuf::from("E:\\Games\\World of Warcraft")
                    );
                    assert_eq!(install_target.flavor, Some(FlavorArg::Retail));
                }
                _ => panic!("expected config plan command"),
            },
            _ => panic!("expected config command"),
        }
    }

    #[test]
    fn parses_top_level_addon_cache_purge() {
        let cli = Cli::parse_from(["hearthsync", "addon", "cache", "purge"]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Cache { command } => match command {
                    AddonCacheCommands::Purge => {}
                    _ => panic!("expected addon cache purge command"),
                },
                _ => panic!("expected addon cache command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_cache_repair() {
        let cli = Cli::parse_from(["hearthsync", "addon", "cache", "repair"]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Cache { command } => match command {
                    AddonCacheCommands::Repair => {}
                    _ => panic!("expected addon cache repair command"),
                },
                _ => panic!("expected addon cache command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_policy_set_with_release_channel() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "policy",
            "set",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--package",
            "WeakAuras",
            "--release-channel",
            "beta",
            "--allow-prerelease",
            "true",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                super::addon::AddonCommands::Policy { command } => match command {
                    AddonPolicyCommands::Set {
                        package,
                        release_channel,
                        allow_prerelease,
                        ..
                    } => {
                        assert_eq!(package, "WeakAuras");
                        assert_eq!(release_channel, Some(ReleaseChannelArg::Beta));
                        assert_eq!(allow_prerelease, Some(true));
                    }
                    _ => panic!("expected addon policy set command"),
                },
                _ => panic!("expected addon policy command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_adopt() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "adopt",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--addon",
            "WeakAuras",
            "--addon",
            "SharedMedia",
            "--package-id",
            "guild-ui",
            "--archive-output",
            "E:\\Exports\\guild-ui.zip",
            "--dry-run",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Adopt {
                    install_target,
                    addon_directories,
                    package_id,
                    archive_output,
                    dry_run,
                } => {
                    assert_eq!(
                        install_target.install,
                        PathBuf::from("E:\\Games\\World of Warcraft")
                    );
                    assert_eq!(addon_directories, vec!["WeakAuras", "SharedMedia"]);
                    assert_eq!(package_id.as_deref(), Some("guild-ui"));
                    assert_eq!(
                        archive_output,
                        Some(PathBuf::from("E:\\Exports\\guild-ui.zip"))
                    );
                    assert!(dry_run);
                }
                _ => panic!("expected addon adopt command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_relink() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "relink",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--name",
            "WeakAuras",
            "--source",
            "github:WeakAuras/WeakAuras2",
            "--dry-run",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Relink {
                    install_target,
                    name,
                    source,
                    dry_run,
                } => {
                    assert_eq!(
                        install_target.install,
                        PathBuf::from("E:\\Games\\World of Warcraft")
                    );
                    assert_eq!(name, "WeakAuras");
                    assert_eq!(source, "github:WeakAuras/WeakAuras2");
                    assert!(dry_run);
                }
                _ => panic!("expected addon relink command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_index_validate() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "index",
            "validate",
            "--file",
            "E:\\Rust\\hearthsync\\addons.index.toml",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Index { command } => match command {
                    AddonIndexCommands::Validate { file } => {
                        assert_eq!(
                            file,
                            PathBuf::from("E:\\Rust\\hearthsync\\addons.index.toml")
                        );
                    }
                    _ => panic!("expected addon index validate command"),
                },
                _ => panic!("expected addon index command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_index_suggest() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "index",
            "suggest",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--file",
            "E:\\Rust\\hearthsync\\addons.index.toml",
            "--name",
            "WeakAuras",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Index { command } => match command {
                    AddonIndexCommands::Suggest {
                        install_target,
                        file,
                        name,
                    } => {
                        assert_eq!(
                            install_target.install,
                            PathBuf::from("E:\\Games\\World of Warcraft")
                        );
                        assert_eq!(
                            file,
                            PathBuf::from("E:\\Rust\\hearthsync\\addons.index.toml")
                        );
                        assert_eq!(name.as_deref(), Some("WeakAuras"));
                    }
                    _ => panic!("expected addon index suggest command"),
                },
                _ => panic!("expected addon index command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_index_attach() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "index",
            "attach",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--file",
            "E:\\Rust\\hearthsync\\addons.index.toml",
            "--name",
            "WeakAuras",
            "--dry-run",
            "--apply-ready-only",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Index { command } => match command {
                    AddonIndexCommands::Attach {
                        install_target,
                        file,
                        name,
                        dry_run,
                        apply_ready_only,
                    } => {
                        assert_eq!(
                            install_target.install,
                            PathBuf::from("E:\\Games\\World of Warcraft")
                        );
                        assert_eq!(
                            file,
                            PathBuf::from("E:\\Rust\\hearthsync\\addons.index.toml")
                        );
                        assert_eq!(name.as_deref(), Some("WeakAuras"));
                        assert!(dry_run);
                        assert!(apply_ready_only);
                    }
                    _ => panic!("expected addon index attach command"),
                },
                _ => panic!("expected addon index command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_index_scaffold() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "index",
            "scaffold",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--file",
            "E:\\Rust\\hearthsync\\addons.index.toml",
            "--index-name",
            "Guild UI",
            "--description",
            "Initial scaffold",
            "--name",
            "WeakAuras",
            "--overwrite",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Index { command } => match command {
                    AddonIndexCommands::Scaffold {
                        install_target,
                        file,
                        index_name,
                        description,
                        name,
                        overwrite,
                    } => {
                        assert_eq!(
                            install_target.install,
                            PathBuf::from("E:\\Games\\World of Warcraft")
                        );
                        assert_eq!(
                            file,
                            PathBuf::from("E:\\Rust\\hearthsync\\addons.index.toml")
                        );
                        assert_eq!(index_name, "Guild UI");
                        assert_eq!(description.as_deref(), Some("Initial scaffold"));
                        assert_eq!(name.as_deref(), Some("WeakAuras"));
                        assert!(overwrite);
                    }
                    _ => panic!("expected addon index scaffold command"),
                },
                _ => panic!("expected addon index command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_addon_index_relink() {
        let cli = Cli::parse_from([
            "hearthsync",
            "addon",
            "index",
            "relink",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
            "--file",
            "E:\\Rust\\hearthsync\\addons.index.toml",
            "--name",
            "WeakAuras",
            "--target",
            "WeakAuras-local",
            "--dry-run",
        ]);

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::Index { command } => match command {
                    AddonIndexCommands::Relink {
                        install_target,
                        file,
                        name,
                        target,
                        dry_run,
                    } => {
                        assert_eq!(
                            install_target.install,
                            PathBuf::from("E:\\Games\\World of Warcraft")
                        );
                        assert_eq!(
                            file,
                            PathBuf::from("E:\\Rust\\hearthsync\\addons.index.toml")
                        );
                        assert_eq!(name, "WeakAuras");
                        assert_eq!(target.as_deref(), Some("WeakAuras-local"));
                        assert!(dry_run);
                    }
                    _ => panic!("expected addon index relink command"),
                },
                _ => panic!("expected addon index command"),
            },
            _ => panic!("expected addon command"),
        }
    }

    #[test]
    fn parses_top_level_global_addon_state_storage() {
        let cli = Cli::parse_from([
            "hearthsync",
            "--addon-state-storage",
            "sidecar",
            "addon",
            "list",
            "--install",
            "E:\\Games\\World of Warcraft",
            "--flavor",
            "retail",
        ]);

        assert_eq!(
            cli.runtime.addon_state_storage,
            Some(AddonStateStorageArg::Sidecar)
        );

        match cli.command {
            Commands::Addon { command } => match command {
                AddonCommands::List { install_target } => {
                    assert_eq!(
                        install_target.install,
                        PathBuf::from("E:\\Games\\World of Warcraft")
                    );
                    assert_eq!(install_target.flavor, Some(FlavorArg::Retail));
                }
                _ => panic!("expected addon list command"),
            },
            _ => panic!("expected addon command"),
        }
    }
}
