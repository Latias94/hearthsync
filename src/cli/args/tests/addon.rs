use super::*;

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
fn parses_top_level_addon_install_with_provider_compatibility_runtime_options() {
    let cli = Cli::parse_from([
        "hearthsync",
        "--addon-state-storage",
        "sidecar",
        "--addon-cache-dir",
        "target/provider-cache",
        "addon",
        "install",
        "--install",
        "E:\\Games\\World of Warcraft",
        "--flavor",
        "retail",
        "--source",
        "wago:qv63A7Gb@vdx1042w",
        "--dry-run",
        "--replace-existing",
    ]);

    assert_eq!(
        cli.runtime.addon_state_storage,
        Some(AddonStateStorageArg::Sidecar)
    );
    assert_eq!(
        cli.runtime.addon_cache_dir,
        Some(PathBuf::from("target/provider-cache"))
    );

    match cli.command {
        Commands::Addon { command } => match command {
            AddonCommands::Install {
                install_target,
                source,
                dry_run,
                replace_existing,
                ..
            } => {
                assert_eq!(
                    install_target.install,
                    PathBuf::from("E:\\Games\\World of Warcraft")
                );
                assert_eq!(source, "wago:qv63A7Gb@vdx1042w");
                assert!(dry_run);
                assert!(replace_existing);
            }
            _ => panic!("expected addon install command"),
        },
        _ => panic!("expected addon command"),
    }
}
