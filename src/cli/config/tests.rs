use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use super::super::bundle::handle_bundle_command;
use super::handle_config_command;
use crate::cli::args::config::ConfigPackageOptions;
use crate::cli::{
    ApplyMappingArgs, BundleCommands, ConfigCommands, FlavorArg, InstallTargetArgs, PlatformArg,
    SharingModeArg,
};
use crate::core::app::{AppRuntime, HostPlatformValue};

#[test]
fn config_cli_runs_export_plan_dry_run_and_apply_with_mapping() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let backup = tempdir().expect("backup temp dir");
    let package_root = create_shareable_config_source(source.path());
    let installation = create_empty_installation(target.path());
    seed_config_target(&installation);
    let runtime = cli_runtime();
    let exported_bundle = source.path().join("shareable-ui.hearthsync.zip");

    handle_config_command(
        true,
        runtime.clone(),
        ConfigCommands::Inspect {
            source: package_root.clone(),
        },
    )
    .expect("inspect config package");

    handle_config_command(
        true,
        runtime.clone(),
        ConfigCommands::Export {
            config_options: sample_config_options(&package_root),
            output: exported_bundle.clone(),
            sharing_mode: SharingModeArg::Public,
            allow_public_sharing_risks: true,
            excluded_wtf_scopes: Vec::new(),
        },
    )
    .expect("export config package");
    assert!(exported_bundle.exists());

    handle_config_command(
        true,
        runtime.clone(),
        ConfigCommands::Plan {
            config_options: sample_config_options(&package_root),
            install_target: install_target(&installation.product_root),
            apply_mapping: apply_mapping(),
        },
    )
    .expect("plan config apply");

    handle_config_command(
        true,
        runtime.clone(),
        ConfigCommands::Apply {
            config_options: sample_config_options(&package_root),
            install_target: install_target(&installation.product_root),
            dry_run: true,
            backup_output: Some(backup.path().to_path_buf()),
            apply_mapping: apply_mapping(),
        },
    )
    .expect("dry-run config apply");
    assert!(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(installation.fonts_dir.join("FRIZQT__.ttf")).expect("font"),
        "old-font"
    );

    handle_config_command(
        true,
        runtime,
        ConfigCommands::Apply {
            config_options: sample_config_options(&package_root),
            install_target: install_target(&installation.product_root),
            dry_run: false,
            backup_output: Some(backup.path().to_path_buf()),
            apply_mapping: apply_mapping(),
        },
    )
    .expect("apply config package");

    assert!(
        fs::read_dir(backup.path())
            .expect("backup dir")
            .next()
            .is_some()
    );
    assert_config_package_applied(&installation);
}

#[test]
fn config_cli_exported_bundle_applies_through_bundle_cli() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let backup = tempdir().expect("backup temp dir");
    let package_root = create_shareable_config_source(source.path());
    let installation = create_empty_installation(target.path());
    seed_config_target(&installation);
    let runtime = cli_runtime();
    let exported_bundle = source.path().join("shareable-ui.hearthsync.zip");

    handle_config_command(
        true,
        runtime.clone(),
        ConfigCommands::Export {
            config_options: sample_config_options(&package_root),
            output: exported_bundle.clone(),
            sharing_mode: SharingModeArg::Public,
            allow_public_sharing_risks: true,
            excluded_wtf_scopes: Vec::new(),
        },
    )
    .expect("export config package");

    handle_bundle_command(
        true,
        runtime.clone(),
        BundleCommands::Inspect {
            bundle: exported_bundle.clone(),
        },
    )
    .expect("inspect exported bundle");
    handle_bundle_command(
        true,
        runtime.clone(),
        BundleCommands::Plan {
            bundle: exported_bundle.clone(),
            install_target: install_target(&installation.product_root),
            apply_mapping: apply_mapping(),
        },
    )
    .expect("plan exported bundle apply");
    handle_bundle_command(
        true,
        runtime.clone(),
        BundleCommands::Unpack {
            bundle: exported_bundle.clone(),
            install_target: install_target(&installation.product_root),
            dry_run: true,
            backup_output: Some(backup.path().to_path_buf()),
            apply_mapping: apply_mapping(),
        },
    )
    .expect("dry-run exported bundle apply");
    assert!(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );

    handle_bundle_command(
        true,
        runtime,
        BundleCommands::Unpack {
            bundle: exported_bundle,
            install_target: install_target(&installation.product_root),
            dry_run: false,
            backup_output: Some(backup.path().to_path_buf()),
            apply_mapping: apply_mapping(),
        },
    )
    .expect("apply exported bundle");

    assert!(
        fs::read_dir(backup.path())
            .expect("backup dir")
            .next()
            .is_some()
    );
    assert_config_package_applied(&installation);
}

#[test]
fn config_cli_apply_rolls_back_when_resource_write_fails() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let backup = tempdir().expect("backup temp dir");
    let package_root = create_shareable_config_source(source.path());
    let installation = create_empty_installation(target.path());
    seed_config_target(&installation);

    let stale_addon = installation.addon_dir.join("WeakAuras").join("Stale.lua");
    let original_stale_addon = fs::read_to_string(&stale_addon).expect("original stale addon");
    let original_config =
        fs::read_to_string(installation.wtf_dir.join("Config.wtf")).expect("original config");
    let original_font =
        fs::read_to_string(installation.fonts_dir.join("FRIZQT__.ttf")).expect("original font");
    let old_texture = installation.interface_dir.join("SharedXML").join("old.blp");
    let original_old_texture = fs::read_to_string(&old_texture).expect("original texture");
    let account_saved_variables = installation
        .wtf_dir
        .join("Account")
        .join("TARGETACC")
        .join("SavedVariables");
    fs::write(&account_saved_variables, "blocking saved variables file")
        .expect("blocking saved variables file");

    let error = handle_config_command(
        true,
        cli_runtime(),
        ConfigCommands::Apply {
            config_options: sample_config_options(&package_root),
            install_target: install_target(&installation.product_root),
            dry_run: false,
            backup_output: Some(backup.path().to_path_buf()),
            apply_mapping: apply_mapping(),
        },
    )
    .expect_err("config apply should fail and roll back");

    assert!(!error.to_string().is_empty());
    assert_eq!(
        fs::read_to_string(&stale_addon).expect("restored stale addon"),
        original_stale_addon
    );
    assert_eq!(
        fs::read_to_string(installation.wtf_dir.join("Config.wtf")).expect("restored config"),
        original_config
    );
    assert_eq!(
        fs::read_to_string(installation.fonts_dir.join("FRIZQT__.ttf")).expect("restored font"),
        original_font
    );
    assert_eq!(
        fs::read_to_string(&old_texture).expect("restored texture"),
        original_old_texture
    );
    assert_eq!(
        fs::read_to_string(&account_saved_variables).expect("blocking file"),
        "blocking saved variables file"
    );
    assert!(
        !installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("Blizzard_Console.lua")
            .exists()
    );
}

struct CliInstallationPaths {
    product_root: PathBuf,
    interface_dir: PathBuf,
    addon_dir: PathBuf,
    wtf_dir: PathBuf,
    fonts_dir: PathBuf,
}

fn cli_runtime() -> AppRuntime {
    AppRuntime::builder()
        .with_host_platform(HostPlatformValue::MacOs)
        .build()
        .expect("runtime")
}

fn sample_config_options(source: &Path) -> ConfigPackageOptions {
    ConfigPackageOptions {
        source: source.to_path_buf(),
        source_flavor: FlavorArg::Retail,
        source_platform: Some(PlatformArg::Windows),
        supported_targets: vec![FlavorArg::Retail],
        package_id: Some("shareable-ui".to_string()),
        package_name: Some("Shareable UI".to_string()),
        created_by: Some("hearthsync-cli-test".to_string()),
        description: Some("shareable config cli fixture".to_string()),
        no_backup: false,
        addons_policy: None,
        wtf_common_policy: None,
        wtf_characters_policy: None,
        fonts_policy: None,
        interface_assets_policy: None,
    }
}

fn install_target(product_root: &Path) -> InstallTargetArgs {
    InstallTargetArgs {
        install: product_root.to_path_buf(),
        flavor: Some(FlavorArg::Retail),
    }
}

fn apply_mapping() -> ApplyMappingArgs {
    ApplyMappingArgs {
        mapping_file: None,
        target_account: Some("TARGETACC".to_string()),
        target_server: Some("Stormrage".to_string()),
        target_character: Some("Targetmage".to_string()),
        selected_accounts: Vec::new(),
        all_accounts: false,
    }
}

fn create_empty_installation(root: &Path) -> CliInstallationPaths {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    CliInstallationPaths {
        product_root,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn seed_config_target(installation: &CliInstallationPaths) {
    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(
        installation.addon_dir.join("WeakAuras").join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon");
    fs::write(installation.wtf_dir.join("Config.wtf"), "SET locale zhCN").expect("config");

    let character_root = installation
        .wtf_dir
        .join("Account")
        .join("TARGETACC")
        .join("Stormrage")
        .join("Targetmage");
    fs::create_dir_all(character_root.join("SavedVariables")).expect("character dir");
    fs::write(character_root.join("StaleCharacter.txt"), "stale-character")
        .expect("stale character");
    fs::write(
        character_root.join("SavedVariables").join("Old.lua"),
        "OldSaved = true",
    )
    .expect("old saved variables");

    fs::write(installation.fonts_dir.join("FRIZQT__.ttf"), "old-font").expect("old font");
    fs::write(installation.fonts_dir.join("OLD__.ttf"), "stale-font").expect("stale font");
    fs::create_dir_all(installation.interface_dir.join("SharedXML")).expect("shared xml");
    fs::write(
        installation.interface_dir.join("SharedXML").join("old.blp"),
        "old-texture",
    )
    .expect("old texture");
}

fn create_shareable_config_source(root: &Path) -> PathBuf {
    let package_root = root.join("AuthorPack");
    let addon_root = package_root
        .join("Interface")
        .join("AddOns")
        .join("WeakAuras");
    fs::create_dir_all(&addon_root).expect("addon dir");
    fs::write(
        addon_root.join("WeakAuras.toc"),
        "## Interface: 110000\n## Title: WeakAuras\n",
    )
    .expect("toc");
    fs::write(
        addon_root.join("WeakAuras.lua"),
        r#"WeakAurasSaved = { ["profileKeys"] = { ["Examplemage - Illidan"] = "Default" } }"#,
    )
    .expect("addon lua");

    fs::create_dir_all(package_root.join("WTF")).expect("wtf dir");
    fs::write(
        package_root.join("WTF").join("Config.wtf"),
        "SET locale enUS",
    )
    .expect("config");
    let root_saved_variables = package_root.join("WTF").join("SavedVariables");
    fs::create_dir_all(&root_saved_variables).expect("root saved variables");
    fs::write(
        root_saved_variables.join("Blizzard_Console.lua"),
        r#"
BlizzardConsoleDB = {
  ["note"] = "console setting stays global",
}
"#,
    )
    .expect("root saved variables file");

    let account_dir = package_root.join("WTF").join("Account").join("ACCOUNT");
    fs::create_dir_all(account_dir.join("SavedVariables")).expect("account saved variables");
    fs::write(
        account_dir.join("SavedVariables").join("Details.lua"),
        load_lua_patch_fixture_text("details_realistic_utf8.lua"),
    )
    .expect("details saved variables");
    fs::write(
        account_dir.join("SavedVariables").join("MeetingStone.lua"),
        load_lua_patch_fixture_text("meetingstone_search_history_context_utf8.lua"),
    )
    .expect("meetingstone saved variables");

    let character_dir = account_dir.join("Illidan").join("Examplemage");
    fs::create_dir_all(character_dir.join("SavedVariables")).expect("character saved variables");
    fs::write(character_dir.join("AddOns.txt"), "WeakAuras: enabled").expect("addons state");
    fs::write(
        character_dir.join("SavedVariables").join("Pawn.lua"),
        r#"
PawnOptions = {
  ["LastPlayerFullName"] = "Examplemage",
  ["LastRealm"] = "Illidan",
}
"#,
    )
    .expect("pawn saved variables");

    fs::create_dir_all(package_root.join("Fonts")).expect("fonts dir");
    fs::write(
        package_root.join("Fonts").join("FRIZQT__.ttf"),
        "author-font",
    )
    .expect("font");
    fs::create_dir_all(package_root.join("Interface").join("SharedXML")).expect("asset dir");
    fs::write(
        package_root
            .join("Interface")
            .join("SharedXML")
            .join("texture.blp"),
        "author-texture",
    )
    .expect("texture");

    package_root
}

fn load_lua_patch_fixture_text(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("core")
            .join("lua_patch")
            .join("testdata")
            .join(name),
    )
    .expect("lua patch fixture")
}

fn assert_config_package_applied(installation: &CliInstallationPaths) {
    let addon_root = installation.addon_dir.join("WeakAuras");
    assert!(addon_root.join("WeakAuras.toc").exists());
    assert!(!addon_root.join("Stale.lua").exists());
    assert_eq!(
        fs::read_to_string(addon_root.join("WeakAuras.lua")).expect("addon lua"),
        "WeakAurasSaved = { [\"profileKeys\"] = { [\"Examplemage - Illidan\"] = \"Default\" } }"
    );
    assert_eq!(
        fs::read_to_string(installation.wtf_dir.join("Config.wtf")).expect("target config"),
        "SET locale zhCN"
    );

    let common_lua = fs::read_to_string(
        installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("Details.lua"),
    )
    .expect("common saved variables");
    assert!(common_lua.contains("Targetmage - Stormrage"));
    assert!(common_lua.contains(r#"["playerName"] = "Targetmage""#));
    assert!(common_lua.contains(r#"["realm"] = "Stormrage""#));
    assert!(common_lua.contains(r#"["lastPlayerName"] = "Examplemage""#));

    let root_lua = fs::read_to_string(
        installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("Blizzard_Console.lua"),
    )
    .expect("root saved variables");
    assert!(root_lua.contains("BlizzardConsoleDB"));

    let character_root = installation
        .wtf_dir
        .join("Account")
        .join("TARGETACC")
        .join("Stormrage")
        .join("Targetmage");
    assert!(!character_root.join("StaleCharacter.txt").exists());
    let character_lua = fs::read_to_string(character_root.join("SavedVariables").join("Pawn.lua"))
        .expect("character saved variables");
    assert!(character_lua.contains(r#""Targetmage""#));
    assert!(character_lua.contains(r#""Stormrage""#));

    assert_eq!(
        fs::read_to_string(installation.fonts_dir.join("FRIZQT__.ttf")).expect("font"),
        "author-font"
    );
    assert!(!installation.fonts_dir.join("OLD__.ttf").exists());
    assert_eq!(
        fs::read_to_string(
            installation
                .interface_dir
                .join("SharedXML")
                .join("texture.blp")
        )
        .expect("texture"),
        "author-texture"
    );
    assert!(
        !installation
            .interface_dir
            .join("SharedXML")
            .join("old.blp")
            .exists()
    );
}
