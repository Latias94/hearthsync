use super::*;

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
