use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::core::app::{
    AppRuntime, ApplyBundleAppRequest, ApplyConfigAppRequest, BundleApplyMappingsValue,
    ConfigPackageAppRequest, ConfigPublicSharingReasonCodeValue, ConfigPublicSharingSeverityValue,
    ConfigPublicSharingStatusValue, ConfigSensitiveWtfFileKindValue, ConfigService,
    ExportConfigBundleAppRequest, ExternalPackageService, ExternalPackageSharingModeValue,
    HostPlatformValue, InspectConfigAppRequest, PlanBundleApplyRequest, PlanConfigApplyAppRequest,
    ResolvedInstallationValue, ResourceApplyPolicyValue, StableAppServices, WowFlavorValue,
    WtfScopeRiskValue, WtfScopeValue,
};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase};

#[test]
fn config_service_inspect_collecting_progress_returns_config_task_events() {
    let temp = tempdir().expect("temp dir");
    let package_root = create_minimal_config_source(temp.path());

    let service = ConfigService::default();
    let run = service
        .inspect_collecting_progress(InspectConfigAppRequest {
            source_path: package_root,
        })
        .expect("inspect with collected progress");

    assert_eq!(run.result.resources.addons, vec!["WeakAuras".to_string()]);
    assert_eq!(
        run.progress
            .iter()
            .map(|event| (event.task, event.phase))
            .collect::<Vec<_>>(),
        vec![
            (TaskKind::ExternalPackageAnalyze, TaskPhase::Preparing),
            (TaskKind::ExternalPackageAnalyze, TaskPhase::Planning),
            (TaskKind::ExternalPackageAnalyze, TaskPhase::Completed),
        ]
    );
}

#[test]
fn config_service_inspects_relative_source_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let package_root = create_minimal_config_source(temp.path());

    let service = ConfigService::with_external_packages(ExternalPackageService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(temp.path().to_path_buf()))
            .build()
            .expect("runtime"),
    ));
    let result = service
        .inspect_collecting_progress(InspectConfigAppRequest {
            source_path: PathBuf::from("AuthorPack"),
        })
        .expect("inspect relative config source")
        .result;

    assert_eq!(result.source_path, package_root);
    assert_eq!(result.resources.addons, vec!["WeakAuras".to_string()]);
}

#[test]
fn config_service_apply_with_callbacks_uses_config_facade_requests() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_root = create_minimal_config_source(source.path());
    let installation = create_empty_installation(target.path());

    let service = ConfigService::default();
    let seen = RefCell::new(Vec::new());
    let cancellation_checks = Cell::new(0usize);
    let result = service
        .apply_with_callbacks(
            ApplyConfigAppRequest {
                config_package: sample_config_package(package_root),
                installation,
                dry_run: true,
                backup_output_path: None,
                apply_mappings: BundleApplyMappingsValue::default(),
            },
            || {
                let next = cancellation_checks.get() + 1;
                cancellation_checks.set(next);
                false
            },
            |event| seen.borrow_mut().push(event),
        )
        .expect("apply with callbacks");

    assert!(result.dry_run);
    assert_eq!(seen.borrow().len(), 3);
    assert!(cancellation_checks.get() >= 2);
}

#[test]
fn config_service_plans_and_applies_shareable_package_with_mapping_backup_and_rewrite() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let backup = tempdir().expect("backup temp dir");
    let package_root = create_shareable_config_source(source.path());
    let installation = create_empty_installation_on_platform(target.path(), HostPlatform::MacOs);
    seed_config_target(&installation);

    let service = ConfigService::default();
    let package = sample_shareable_config_package(package_root);
    let mappings = BundleApplyMappingsValue {
        target_account: Some("TARGETACC".to_string()),
        target_server: Some("Stormrage".to_string()),
        target_character: Some("Targetmage".to_string()),
        ..BundleApplyMappingsValue::default()
    };

    let plan = service
        .plan_apply_collecting_progress(PlanConfigApplyAppRequest {
            config_package: package.clone(),
            installation: installation.clone(),
            apply_mappings: mappings.clone(),
        })
        .expect("plan config apply")
        .result;

    assert_eq!(plan.inspection.resources.addons, vec!["WeakAuras"]);
    assert_eq!(plan.inspection.resources.wtf_character_count, 1);
    assert!(plan.manifest.apply.create_backup);
    assert_eq!(
        plan.group_policies.wtf_characters.policy,
        ResourceApplyPolicyValue::ReplaceSelected
    );
    assert!(plan.summary.files_to_add > 0);
    assert!(plan.summary.paths_to_remove >= 3);
    assert!(
        plan.inspection
            .summary
            .wtf_scopes
            .iter()
            .any(|scope| scope.scope == WtfScopeValue::AccountSavedVariables
                && scope.risk == WtfScopeRiskValue::High)
    );
    assert!(
        plan.inspection
            .summary
            .wtf_scopes
            .iter()
            .any(
                |scope| scope.scope == WtfScopeValue::CharacterSavedVariables
                    && scope.risk == WtfScopeRiskValue::Medium
            )
    );
    assert_eq!(
        plan.inspection.summary.source_identities.source_accounts,
        vec!["ACCOUNT"]
    );
    assert!(
        plan.inspection
            .summary
            .sensitive_wtf_files
            .iter()
            .any(
                |file| file.kind == ConfigSensitiveWtfFileKindValue::SavedVariables
                    && file.severity == ConfigPublicSharingSeverityValue::ReviewRequired
                    && file.count == 4
            )
    );
    assert!(
        plan.inspection
            .summary
            .sensitive_wtf_files
            .iter()
            .any(
                |file| file.kind == ConfigSensitiveWtfFileKindValue::AddonEnablement
                    && file.severity == ConfigPublicSharingSeverityValue::Advisory
                    && file.count == 1
            )
    );
    assert_eq!(
        plan.inspection
            .summary
            .source_identities
            .source_characters
            .len(),
        1
    );
    assert_eq!(
        plan.inspection.summary.public_sharing.status,
        ConfigPublicSharingStatusValue::ReviewRequired
    );
    assert!(!plan.inspection.summary.public_sharing.public_ready);
    assert_eq!(
        plan.inspection.summary.public_sharing.review_required_count,
        5
    );
    assert_eq!(plan.inspection.summary.public_sharing.advisory_count, 1);
    assert!(
        plan.inspection
            .summary
            .public_sharing
            .reasons
            .iter()
            .any(
                |reason| reason.severity == ConfigPublicSharingSeverityValue::ReviewRequired
                    && reason.code == ConfigPublicSharingReasonCodeValue::HighRiskWtfScope
                    && reason.count == 3
            )
    );
    assert!(
        plan.inspection
            .summary
            .public_sharing
            .reasons
            .iter()
            .any(
                |reason| reason.severity == ConfigPublicSharingSeverityValue::ReviewRequired
                    && reason.code == ConfigPublicSharingReasonCodeValue::SourceCharacterIdentity
                    && reason.count == 2
            )
    );
    assert!(
        plan.inspection
            .summary
            .public_sharing
            .reasons
            .iter()
            .any(
                |reason| reason.severity == ConfigPublicSharingSeverityValue::ReviewRequired
                    && reason.code == ConfigPublicSharingReasonCodeValue::SensitiveWtfFile
                    && reason.count == 4
            )
    );

    let applied = service
        .apply_collecting_progress(ApplyConfigAppRequest {
            config_package: package,
            installation: installation.clone(),
            dry_run: false,
            backup_output_path: Some(backup.path().to_path_buf()),
            apply_mappings: mappings,
        })
        .expect("apply config package")
        .result;

    assert!(!applied.dry_run);
    assert!(applied.backup_path.is_some());
    assert_eq!(
        applied.backup_path.as_deref().and_then(Path::parent),
        Some(backup.path())
    );
    assert!(applied.written_files > 0);
    assert!(applied.rewritten_files >= 3);

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
    assert!(common_lua.contains("中文提示：保留原样"));

    let meetingstone_lua = fs::read_to_string(
        installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("MeetingStone.lua"),
    )
    .expect("meetingstone saved variables");
    assert!(meetingstone_lua.contains(r#"["Targetmage - Stormrage"] = {"#));
    assert!(meetingstone_lua.contains(r#"["Examplemage - Illidan"] = "activity label text""#));

    let root_lua = fs::read_to_string(
        installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("Blizzard_Console.lua"),
    )
    .expect("root saved variables");
    assert!(root_lua.contains("BlizzardConsoleDB"));
    assert!(root_lua.contains("console setting stays global"));

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

#[test]
fn config_service_rolls_back_shareable_package_apply_when_resource_write_fails() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let backup = tempdir().expect("backup temp dir");
    let package_root = create_shareable_config_source(source.path());
    let installation = create_empty_installation_on_platform(target.path(), HostPlatform::MacOs);
    seed_config_target(&installation);

    let stale_addon = installation.addon_dir.join("WeakAuras").join("Stale.lua");
    let original_stale_addon = fs::read_to_string(&stale_addon).expect("original stale addon");
    let original_config =
        fs::read_to_string(installation.wtf_dir.join("Config.wtf")).expect("original config");
    let original_font =
        fs::read_to_string(installation.fonts_dir.join("FRIZQT__.ttf")).expect("original font");
    let old_texture = installation.interface_dir.join("SharedXML").join("old.blp");
    let original_old_texture = fs::read_to_string(&old_texture).expect("original old texture");
    let character_root = installation
        .wtf_dir
        .join("Account")
        .join("TARGETACC")
        .join("Stormrage")
        .join("Targetmage");

    let account_saved_variables = installation
        .wtf_dir
        .join("Account")
        .join("TARGETACC")
        .join("SavedVariables");
    fs::write(&account_saved_variables, "blocking saved variables file")
        .expect("blocking account saved variables file");

    let service = ConfigService::default();
    let error = service
        .apply_collecting_progress(ApplyConfigAppRequest {
            config_package: sample_shareable_config_package(package_root),
            installation: installation.clone(),
            dry_run: false,
            backup_output_path: Some(backup.path().to_path_buf()),
            apply_mappings: BundleApplyMappingsValue {
                target_account: Some("TARGETACC".to_string()),
                target_server: Some("Stormrage".to_string()),
                target_character: Some("Targetmage".to_string()),
                ..BundleApplyMappingsValue::default()
            },
        })
        .expect_err("config apply should fail and roll back");

    assert!(error.to_string().contains("rollback restored"));
    assert_eq!(
        fs::read_to_string(&stale_addon).expect("restored stale addon"),
        original_stale_addon
    );
    assert!(
        !installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(installation.wtf_dir.join("Config.wtf")).expect("restored config"),
        original_config
    );
    assert_eq!(
        fs::read_to_string(installation.fonts_dir.join("FRIZQT__.ttf")).expect("restored font"),
        original_font
    );
    assert!(character_root.join("StaleCharacter.txt").exists());
    assert!(
        character_root
            .join("SavedVariables")
            .join("Old.lua")
            .exists()
    );
    assert!(
        !character_root
            .join("SavedVariables")
            .join("Pawn.lua")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(&account_saved_variables).expect("restored blocking file"),
        "blocking saved variables file"
    );
    assert_eq!(
        fs::read_to_string(&old_texture).expect("restored old texture"),
        original_old_texture
    );
    assert!(
        !installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("Blizzard_Console.lua")
            .exists()
    );
    assert!(
        !installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );
}

#[test]
fn stable_app_exports_config_bundle_and_applies_exported_bundle() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let backup = tempdir().expect("backup temp dir");
    let export = tempdir().expect("export temp dir");
    let package_root = create_shareable_config_source(source.path());
    let installation = create_empty_installation_on_platform(target.path(), HostPlatform::MacOs);
    seed_config_target(&installation);

    let app = StableAppServices::new();
    let export_path = export.path().join("shareable-ui.hearthsync.zip");
    let mut config_package = sample_shareable_config_package(package_root);
    config_package.output_path = Some(export_path.clone());

    let exported = app
        .export_config(ExportConfigBundleAppRequest {
            config_package,
            sharing_mode: ExternalPackageSharingModeValue::Private,
            allow_public_sharing_risks: false,
            excluded_wtf_scopes: Vec::new(),
        })
        .expect("export config bundle");
    let exported = exported.as_ref().clone();

    assert_eq!(exported.bundle.archive_path, export_path);
    assert!(exported.bundle.archive_path.is_file());
    assert_eq!(exported.manifest.package.name, "Shareable UI");
    assert!(
        exported
            .inspection
            .summary
            .sensitive_wtf_files
            .iter()
            .any(
                |file| file.kind == ConfigSensitiveWtfFileKindValue::SavedVariables
                    && file.count == 4
            )
    );

    let mappings = BundleApplyMappingsValue {
        target_account: Some("TARGETACC".to_string()),
        target_server: Some("Stormrage".to_string()),
        target_character: Some("Targetmage".to_string()),
        ..BundleApplyMappingsValue::default()
    };
    let plan = app
        .plan_bundle_apply(PlanBundleApplyRequest {
            bundle_path: export_path.clone(),
            installation: installation.clone(),
            apply_mappings: mappings.clone(),
        })
        .expect("plan exported bundle apply");

    assert!(plan.summary.files_to_add > 0);
    assert!(plan.summary.paths_to_remove >= 3);

    let applied = app
        .apply_bundle(ApplyBundleAppRequest {
            bundle_path: export_path,
            installation: installation.clone(),
            dry_run: false,
            backup_output_path: Some(backup.path().to_path_buf()),
            apply_mappings: mappings,
        })
        .expect("apply exported bundle")
        .result;

    assert!(!applied.dry_run);
    assert!(applied.backup_path.is_some());
    assert!(applied.written_files > 0);
    assert!(applied.rewritten_files >= 3);
    assert!(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
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
    assert!(common_lua.contains("中文提示：保留原样"));
    let meetingstone_lua = fs::read_to_string(
        installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("MeetingStone.lua"),
    )
    .expect("meetingstone saved variables");
    assert!(meetingstone_lua.contains(r#"["Targetmage - Stormrage"] = {"#));
    let root_lua = fs::read_to_string(
        installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("Blizzard_Console.lua"),
    )
    .expect("root saved variables");
    assert!(root_lua.contains("BlizzardConsoleDB"));
    let character_lua = fs::read_to_string(
        installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("Stormrage")
            .join("Targetmage")
            .join("SavedVariables")
            .join("Pawn.lua"),
    )
    .expect("character saved variables");
    assert!(character_lua.contains(r#""Targetmage""#));
    assert!(character_lua.contains(r#""Stormrage""#));
}

fn sample_config_package(source_path: PathBuf) -> ConfigPackageAppRequest {
    ConfigPackageAppRequest {
        source_path,
        source_flavor: WowFlavorValue::Retail,
        source_platform: Some(HostPlatformValue::Windows),
        supported_targets: vec![WowFlavorValue::Retail],
        output_path: None,
        package_id: None,
        package_name: None,
        created_by: None,
        description: None,
        apply_defaults: None,
    }
}

fn sample_shareable_config_package(source_path: PathBuf) -> ConfigPackageAppRequest {
    ConfigPackageAppRequest {
        source_path,
        source_flavor: WowFlavorValue::Retail,
        source_platform: Some(HostPlatformValue::Windows),
        supported_targets: vec![WowFlavorValue::Retail],
        output_path: None,
        package_id: Some("shareable-ui".to_string()),
        package_name: Some("Shareable UI".to_string()),
        created_by: Some("hearthsync-test".to_string()),
        description: Some("shareable config fixture".to_string()),
        apply_defaults: None,
    }
}

fn create_minimal_config_source(root: &Path) -> PathBuf {
    let package_root = root.join("AuthorPack");
    let addon_root = package_root.join("WeakAuras");
    fs::create_dir_all(&addon_root).expect("addon dir");
    fs::write(
        addon_root.join("WeakAuras.toc"),
        "## Interface: 110000\n## Title: WeakAuras\n",
    )
    .expect("toc");
    fs::write(addon_root.join("WeakAuras.lua"), "WeakAurasSaved = {}").expect("lua");
    package_root
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

fn create_empty_installation(root: &Path) -> ResolvedInstallationValue {
    create_empty_installation_on_platform(root, HostPlatform::Windows)
}

fn create_empty_installation_on_platform(
    root: &Path,
    platform: HostPlatform,
) -> ResolvedInstallationValue {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    ResolvedInstallationValue::from_domain(crate::core::install::DetectedFlavorInstallation {
        platform,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    })
}

fn seed_config_target(installation: &ResolvedInstallationValue) {
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
