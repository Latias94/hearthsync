use std::fs;
use std::io::Write;

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::{
    BundleAddonLockApplyRequest, BundleApplyMappings, PackBundleRequest, UnpackBundleRequest,
    apply_bundle_addon_lock, inspect_bundle, pack_bundle, plan_bundle_addon_lock,
    plan_bundle_apply, unpack_bundle,
};
use crate::core::addon::lock::plan_addon_lock_sync;
use crate::core::addon::{InstallAddonRequest, install_addon};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};

#[test]
fn pack_bundle_writes_normalized_layout() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let bundle_path = temp.path().join("bundle.zip");

    let bundle = pack_bundle(PackBundleRequest {
        installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    assert_eq!(bundle.archive_path, bundle_path);

    let file = fs::File::open(bundle.archive_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");

    assert!(archive.by_name("manifest.toml").is_ok());
    assert!(archive.by_name("addons/WeakAuras/WeakAuras.toc").is_ok());
    assert!(archive.by_name("addons/WeakAuras/WeakAuras.lua").is_ok());
    assert!(archive.by_name("wtf/common/Config.wtf").is_ok());
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt")
            .is_ok()
    );
    assert!(archive.by_name("fonts/FRIZQT__.ttf").is_ok());
    assert!(archive.by_name("interface/SharedXML/texture.blp").is_ok());
}

#[test]
fn unpack_bundle_restores_files_and_creates_backup() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path: bundle_path.clone(),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(result.bundle_path, bundle_path);
    assert!(result.written_files > 0);
    assert!(
        result
            .backup_path
            .as_ref()
            .is_some_and(|path| path.exists())
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(target_installation.wtf_dir.join("Config.wtf").exists());
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("AddOns.txt")
            .exists()
    );
    assert!(target_installation.fonts_dir.join("FRIZQT__.ttf").exists());
    assert!(
        target_installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );

    let inspection = inspect_bundle(&result.bundle_path).expect("inspect bundle");
    assert_eq!(inspection.entries.addons, 2);
    assert_eq!(inspection.entries.fonts, 1);
}

#[test]
fn plan_bundle_apply_discovers_local_accounts_and_selected_accounts() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables"),
    )
    .expect("account a");
    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables"),
    )
    .expect("account b");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            selected_accounts: vec!["ACC_A".to_string()],
            ..BundleApplyMappings::default()
        },
    )
    .expect("plan bundle");

    assert_eq!(plan.discovered_accounts.len(), 2);
    assert_eq!(plan.selected_target_accounts, vec!["ACC_A".to_string()]);
    assert!(plan.summary.files_to_add > 0);
    assert!(plan.operations.iter().any(|item| {
        item.group == super::ApplyGroup::WtfCommon
            && item.target_account.as_deref() == Some("ACC_A")
    }));
}

#[test]
fn keep_original_character_mode_ignores_target_identity_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::KeepOriginal;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            target_account: Some("TARGETACC".to_string()),
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            ..BundleApplyMappings::default()
        },
    )
    .expect("plan bundle");

    assert_eq!(plan.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(plan.character_mappings.len(), 1);
    assert_eq!(plan.character_mappings[0].target_account, "ACCOUNT");
    assert_eq!(plan.character_mappings[0].target_server, "Illidan");
    assert_eq!(plan.character_mappings[0].target_character, "Examplemage");
}

#[test]
fn explicit_character_mode_requires_resolved_target_identity() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect_err("explicit mode should require a resolved target identity");

    assert!(
        error
            .to_string()
            .contains("explicit character mode requires a fully resolved target identity")
    );
    assert!(error.to_string().contains("--mapping-file"));
}

#[test]
fn prompt_character_mode_requires_resolved_target_identity() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Prompt;
    manifest.resources.wtf_characters[0].target_hint = Some("Map to your main".to_string());

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect_err("prompt mode should require caller-provided mappings");

    assert!(
        error
            .to_string()
            .contains("current CLI does not prompt automatically")
    );
    assert!(error.to_string().contains("Map to your main"));
}

#[test]
fn multi_character_explicit_mode_rejects_global_target_identity_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;
    manifest.resources.wtf_characters.push(CharacterResource {
        source_account: Some("ACCOUNT".to_string()),
        source_server: "Illidan".to_string(),
        source_character: "Altmage".to_string(),
        target_hint: None,
    });
    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Altmage"),
    )
    .expect("alt character");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Altmage")
            .join("AddOns.txt"),
        "Altmage",
    )
    .expect("alt addons");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            ..BundleApplyMappings::default()
        },
    )
    .expect_err("multi-character explicit mode should reject global target identity");

    assert!(error.to_string().contains("exactly one character"));
    assert!(error.to_string().contains("--mapping-file"));
}

#[test]
fn bundle_apply_plan_does_not_expose_execution_only_fields() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest_with_rewrite(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    let operations = serde_json::to_value(&plan)
        .expect("serialize plan")
        .get("operations")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("operations array");

    assert!(!operations.is_empty());
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("staged_path").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrites").is_none())
    );
}

#[test]
fn bundle_apply_plan_uses_explicit_resource_group_order() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    let mut groups = Vec::new();
    for operation in plan
        .operations
        .iter()
        .filter(|operation| operation.action == super::ApplyAction::Add)
    {
        if groups.last().copied() != Some(operation.group) {
            groups.push(operation.group);
        }
    }

    assert_eq!(
        groups,
        vec![
            super::ApplyGroup::Addons,
            super::ApplyGroup::InterfaceAssets,
            super::ApplyGroup::Fonts,
            super::ApplyGroup::WtfCommon,
            super::ApplyGroup::WtfCharacters,
        ]
    );
}

#[test]
fn plan_bundle_apply_classifies_wtf_scopes_and_account_root_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let source_account_dir = source_installation.wtf_dir.join("Account").join("ACCOUNT");
    let source_character_dir = source_account_dir.join("Illidan").join("Examplemage");

    fs::write(
        source_account_dir.join("account-settings.wtf"),
        "account root",
    )
    .expect("account root file");
    fs::write(source_account_dir.join("config-cache.wtf"), "account cache")
        .expect("account cache file");
    fs::create_dir_all(source_character_dir.join("SavedVariables"))
        .expect("character saved variables");
    fs::write(
        source_character_dir.join("SavedVariables").join("Pawn.lua"),
        "PawnDB = {}",
    )
    .expect("character saved variable");
    fs::write(
        source_character_dir.join("config-cache.wtf"),
        "character cache",
    )
    .expect("character cache file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let file = fs::File::open(&bundle_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/account-settings.wtf")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/config-cache.wtf")
            .is_ok()
    );

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    let scope_for = |archive_name: &str| {
        plan.operations
            .iter()
            .find(|operation| operation.archive_name == archive_name)
            .and_then(|operation| operation.wtf_scope)
    };

    assert_eq!(
        scope_for("wtf/common/Config.wtf"),
        Some(super::WtfScope::GlobalConfig)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/account-settings.wtf"),
        Some(super::WtfScope::AccountRootFile)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
        Some(super::WtfScope::AccountSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua"),
        Some(super::WtfScope::CharacterSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt"),
        Some(super::WtfScope::CharacterState)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/config-cache.wtf"),
        Some(super::WtfScope::CacheLike)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/config-cache.wtf"),
        Some(super::WtfScope::CacheLike)
    );
}

#[test]
fn plan_bundle_apply_skips_identical_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    assert_eq!(plan.summary.files_to_add, 0);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert!(plan.summary.files_to_skip > 0);
    assert_eq!(plan.summary.files_to_skip, plan.operations.len());
    assert_eq!(
        plan.group_policies.addons.policy,
        ResourceApplyPolicy::Merge
    );
    assert!(
        plan.operations
            .iter()
            .all(|operation| operation.action == super::ApplyAction::Skip)
    );
}

#[test]
fn unpack_bundle_applies_character_mapping_and_lua_rewrite() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest_with_rewrite();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            target_account: Some("TARGETACC".to_string()),
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            selected_accounts: Vec::new(),
            all_accounts: false,
            characters: Vec::new(),
        },
    })
    .expect("unpack bundle");

    assert_eq!(result.character_mappings.len(), 1);
    assert!(result.rewritten_files >= 2);
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("Stormrage")
            .join("Targetmage")
            .join("SavedVariables")
            .join("Pawn.lua")
            .exists()
    );

    let common_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("Details.lua"),
    )
    .expect("common lua");
    assert!(common_lua.contains("Targetmage - Stormrage"));
    assert!(common_lua.contains("Default.Stormrage.Targetmage"));

    let character_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("Stormrage")
            .join("Targetmage")
            .join("SavedVariables")
            .join("Pawn.lua"),
    )
    .expect("character lua");
    assert!(character_lua.contains(r#""Targetmage""#));
    assert!(character_lua.contains(r#""Stormrage""#));

    let addon_lua = fs::read_to_string(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.lua"),
    )
    .expect("addon lua");
    assert!(addon_lua.contains("Examplemage - Illidan"));
    assert!(!addon_lua.contains("Targetmage - Stormrage"));
}

#[test]
fn unpack_bundle_replicates_common_wtf_to_selected_accounts() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables"),
    )
    .expect("account a");
    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables"),
    )
    .expect("account b");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            selected_accounts: vec!["ACC_A".to_string(), "ACC_B".to_string()],
            ..BundleApplyMappings::default()
        },
    })
    .expect("unpack bundle");

    assert_eq!(
        result.selected_target_accounts,
        vec!["ACC_A".to_string(), "ACC_B".to_string()]
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
}

#[test]
fn unpack_bundle_rolls_back_when_apply_fails() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    fs::write(
        source_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "## Interface: 120000",
    )
    .expect("updated toc");
    fs::write(
        source_installation
            .addon_dir
            .join("WeakAuras")
            .join("Extra.lua"),
        "print('extra')",
    )
    .expect("extra addon file");
    fs::write(
        source_installation.wtf_dir.join("Config.wtf"),
        "SET locale zhCN",
    )
    .expect("updated config");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let original_toc = fs::read_to_string(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
    )
    .expect("original toc");
    let original_config = fs::read_to_string(target_installation.wtf_dir.join("Config.wtf"))
        .expect("original config");

    let shared_xml = target_installation.interface_dir.join("SharedXML");
    fs::remove_dir_all(&shared_xml).expect("remove shared xml");
    fs::write(&shared_xml, "blocking file").expect("blocking file");

    let error = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect_err("unpack should fail");

    assert!(error.to_string().contains("rollback restored"));
    assert_eq!(
        fs::read_to_string(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("restored toc"),
        original_toc
    );
    assert_eq!(
        fs::read_to_string(target_installation.wtf_dir.join("Config.wtf"))
            .expect("restored config"),
        original_config
    );
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Extra.lua")
            .exists()
    );
    assert!(shared_xml.is_file());
    assert_eq!(
        fs::read_to_string(&shared_xml).expect("restored blocking file"),
        "blocking file"
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );
}

#[test]
fn unpack_bundle_dry_run_does_not_write_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: true,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("dry run");

    assert!(result.dry_run);
    assert!(result.planned_files > 0);
    assert_eq!(result.written_files, 0);
    assert!(result.backup_path.is_none());
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn preserve_policy_plans_without_writing_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_common = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_characters = ResourceApplyPolicy::Preserve;
    manifest.apply.fonts = ResourceApplyPolicy::Preserve;
    manifest.apply.interface_assets = ResourceApplyPolicy::Preserve;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.files_to_preserve > 0);
    assert_eq!(plan.summary.files_to_add, 0);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.summary.files_to_preserve, plan.operations.len());
    assert!(
        plan.operations
            .iter()
            .all(|operation| operation.action == super::ApplyAction::Preserve)
    );

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(result.written_files, 0);
    assert_eq!(result.plan_summary.files_to_preserve, result.planned_files);
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(!target_installation.wtf_dir.join("Config.wtf").exists());
}

#[test]
fn share_policy_preserves_existing_target_files_and_adds_missing_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_common = ResourceApplyPolicy::Share;
    manifest.apply.wtf_characters = ResourceApplyPolicy::Preserve;
    manifest.apply.fonts = ResourceApplyPolicy::Preserve;
    manifest.apply.interface_assets = ResourceApplyPolicy::Preserve;

    fs::write(
        target_installation.wtf_dir.join("Config.wtf"),
        "SET locale zhCN",
    )
    .expect("existing target config");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/Config.wtf"
            && operation.action == super::ApplyAction::Preserve
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"
            && operation.action == super::ApplyAction::Add
    }));

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(
        fs::read_to_string(target_installation.wtf_dir.join("Config.wtf")).expect("target config"),
        "SET locale zhCN"
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(result.plan_summary.files_to_preserve >= 1);
    assert!(result.written_files >= 1);
}

#[test]
fn mirror_policy_removes_existing_addon_root_before_copy() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Mirror;

    fs::write(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.paths_to_remove >= 1);
    assert!(plan.operations.iter().any(|operation| {
        operation.action == super::ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert!(result.written_files > 0);
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn sync_policy_alias_removes_existing_addon_root_before_copy() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Sync;

    fs::write(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.paths_to_remove >= 1);
    assert!(plan.operations.iter().any(|operation| {
        operation.action == super::ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));

    unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn pack_bundle_embeds_addon_lock_and_indexes_as_sidecar_metadata() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), false);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let archive_path = source.path().join("WeakAuras.zip");
    let index_path = source.path().join("addon-index.toml");

    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    install_addon(InstallAddonRequest {
        installation: source_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(source.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = { kind = "local_archive", path = "WeakAuras.zip" }
"#,
    )
    .expect("index");

    let mut manifest = sample_manifest();
    manifest.resources.addons = Vec::new();
    manifest.resources.wtf_common = false;
    manifest.resources.wtf_characters = Vec::new();
    manifest.resources.fonts = false;
    manifest.resources.interface_assets = Vec::new();
    manifest.resources.addon_lock = true;
    manifest.resources.addon_indexes = vec!["addon-index.toml".to_string()];
    manifest.mapping.character_mode = CharacterMappingMode::KeepOriginal;
    manifest.apply.addons = ResourceApplyPolicy::Mirror;

    let bundle = pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: Some(source.path().to_path_buf()),
    })
    .expect("pack bundle");

    let file = fs::File::open(&bundle.archive_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");
    assert!(archive.by_name("metadata/addons/lock.toml").is_ok());
    assert!(archive.by_name("metadata/addons/sources.toml").is_ok());
    assert!(
        archive
            .by_name("metadata/addons/sources/addons-weakauras.zip")
            .is_ok()
    );
    assert!(
        archive
            .by_name("metadata/addons/indexes/addon-index.toml")
            .is_ok()
    );

    let inspection = inspect_bundle(&bundle.archive_path).expect("inspect bundle");
    assert_eq!(inspection.entries.metadata, 5);
    assert_eq!(
        inspection.manifest.apply.addons,
        ResourceApplyPolicy::Mirror
    );
    fs::remove_file(&archive_path).expect("remove original addon source");

    unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    let sidecar_root = target_installation
        .addon_dir
        .join(".hearthsync")
        .join("bundles")
        .join("test-ui");
    assert!(sidecar_root.join("addons").join("lock.toml").exists());
    assert!(
        sidecar_root
            .join("addons")
            .join("indexes")
            .join("addon-index.toml")
            .exists()
    );

    let sidecar_plan = plan_addon_lock_sync(
        &target_installation,
        Some(&sidecar_root.join("addons").join("lock.toml")),
    )
    .expect("sidecar addon plan");
    assert_eq!(sidecar_plan.install_count, 1);
    assert_eq!(sidecar_plan.blocked_count, 0);

    let addon_plan =
        plan_bundle_addon_lock(&bundle.archive_path, &target_installation).expect("addon plan");
    assert_eq!(addon_plan.plan.install_count, 1);
    assert_eq!(addon_plan.plan.update_count, 0);
    assert_eq!(addon_plan.plan.remove_count, 0);
    assert_eq!(addon_plan.plan.blocked_count, 0);

    let addon_apply = apply_bundle_addon_lock(BundleAddonLockApplyRequest {
        bundle_path: bundle.archive_path,
        installation: target_installation.clone(),
        backup_output_path: Some(target.path().join("addon-backups")),
        replace_existing: false,
    })
    .expect("addon apply");
    assert!(addon_apply.apply.verification.matches);
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

fn create_fixture_installation(
    root: &std::path::Path,
    with_content: bool,
) -> DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon root");
    fs::create_dir_all(&wtf_dir).expect("wtf root");
    fs::create_dir_all(&fonts_dir).expect("fonts root");

    if with_content {
        fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
        fs::write(
            addon_dir.join("WeakAuras").join("WeakAuras.toc"),
            "## Interface: 110000",
        )
        .expect("toc");
        fs::write(
            addon_dir.join("WeakAuras").join("WeakAuras.lua"),
            r#"
WeakAurasSaved = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["player"] = "Examplemage",
}
"#,
        )
        .expect("addon lua");

        fs::write(wtf_dir.join("Config.wtf"), "SET locale enUS").expect("config");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("SavedVariables"),
        )
        .expect("saved variables");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("SavedVariables")
                .join("Details.lua"),
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
        .expect("saved variable");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage"),
        )
        .expect("character");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("SavedVariables"),
        )
        .expect("character saved variables");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("AddOns.txt"),
            "WeakAuras: enabled",
        )
        .expect("addons state");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("SavedVariables")
                .join("Pawn.lua"),
            r#"
PawnOptions = {
  ["LastPlayerFullName"] = "Examplemage",
  ["LastRealm"] = "Illidan",
}
"#,
        )
        .expect("character lua");

        fs::write(fonts_dir.join("FRIZQT__.ttf"), "font").expect("font");
        fs::create_dir_all(interface_dir.join("SharedXML")).expect("asset dir");
        fs::write(
            interface_dir.join("SharedXML").join("texture.blp"),
            "texture",
        )
        .expect("asset");
    }

    DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn create_addon_archive(path: &std::path::Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(
            name.replace('\\', "/"),
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start file");
        zip.write_all(content.as_bytes()).expect("write file");
    }
    zip.finish().expect("finish zip");
}

fn sample_manifest() -> BundleManifest {
    BundleManifest {
        schema_version: 1,
        package: PackageMetadata {
            id: "test-ui".to_string(),
            name: "Test UI".to_string(),
            created_by: "test".to_string(),
            description: None,
        },
        source: SourceInstallation {
            flavor: WowFlavor::Retail,
            platform: None,
            exported_at: None,
            supported_targets: vec![WowFlavor::Retail],
        },
        resources: BundleResources {
            addons: vec!["WeakAuras".to_string()],
            wtf_common: true,
            wtf_characters: vec![CharacterResource {
                source_account: Some("ACCOUNT".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Examplemage".to_string(),
                target_hint: None,
            }],
            fonts: true,
            interface_assets: vec!["SharedXML".to_string()],
            addon_lock: false,
            addon_indexes: Vec::new(),
        },
        mapping: MappingRules {
            character_mode: CharacterMappingMode::KeepOriginal,
            rewrite_profile_keys: false,
            rewrite_identity_strings: false,
            allow_cross_platform: true,
        },
        apply: ApplyDefaults {
            create_backup: true,
            addons: ResourceApplyPolicy::Merge,
            wtf_common: ResourceApplyPolicy::Merge,
            wtf_characters: ResourceApplyPolicy::Merge,
            fonts: ResourceApplyPolicy::Merge,
            interface_assets: ResourceApplyPolicy::Merge,
        },
    }
}

fn sample_manifest_with_rewrite() -> BundleManifest {
    let mut manifest = sample_manifest();
    manifest.mapping.rewrite_profile_keys = true;
    manifest.mapping.rewrite_identity_strings = true;
    manifest
}
