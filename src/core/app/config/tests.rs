use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::core::app::{
    AppRuntime, ApplyConfigAppRequest, BundleApplyMappingsValue, ConfigPackageAppRequest,
    ConfigService, ExternalPackageService, HostPlatformValue, InspectConfigAppRequest,
    PlanConfigApplyAppRequest, ResolvedInstallationValue, ResourceApplyPolicyValue, WowFlavorValue,
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
    assert_eq!(
        plan.inspection
            .summary
            .source_identities
            .source_characters
            .len(),
        1
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
    assert!(applied.rewritten_files >= 2);

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
    assert!(common_lua.contains("Default.Stormrage.Targetmage"));

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
    let account_dir = package_root.join("WTF").join("Account").join("ACCOUNT");
    fs::create_dir_all(account_dir.join("SavedVariables")).expect("account saved variables");
    fs::write(
        account_dir.join("SavedVariables").join("Details.lua"),
        r#"
DetailsDB = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["profiles"] = {
    ["Default.Illidan.Examplemage"] = {},
  },
}
"#,
    )
    .expect("details saved variables");
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
