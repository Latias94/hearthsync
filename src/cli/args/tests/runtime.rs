use super::*;

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
        "--addon-cache-repair-remote-policy",
        "local-only",
        "--addon-search-cache-ttl-secs",
        "0",
        "runtime",
    ]);

    assert_eq!(
        cli.runtime.addon_cache_dir,
        Some(PathBuf::from("E:\\Cache"))
    );
    assert_eq!(cli.runtime.addon_http_no_validator_window_secs, Some(120));
    assert_eq!(
        cli.runtime.addon_cache_repair_remote_policy,
        Some(AddonCacheRepairRemotePolicyArg::LocalOnly)
    );
    assert_eq!(cli.runtime.addon_search_cache_ttl_secs, Some(0));
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
