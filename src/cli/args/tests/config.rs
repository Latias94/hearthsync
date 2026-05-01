use super::*;

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
