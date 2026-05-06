use super::*;

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
fn parses_top_level_addon_index_search() {
    let cli = Cli::parse_from([
        "hearthsync",
        "addon",
        "index",
        "search",
        "--file",
        "E:\\Rust\\hearthsync\\catalog\\community-addon-index.toml",
        "--query",
        "ElvUI",
        "--limit",
        "5",
    ]);

    match cli.command {
        Commands::Addon { command } => match command {
            AddonCommands::Index { command } => match command {
                AddonIndexCommands::Search { file, query, limit } => {
                    assert_eq!(
                        file,
                        PathBuf::from("E:\\Rust\\hearthsync\\catalog\\community-addon-index.toml")
                    );
                    assert_eq!(query, "ElvUI");
                    assert_eq!(limit, 5);
                }
                _ => panic!("expected addon index search command"),
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
