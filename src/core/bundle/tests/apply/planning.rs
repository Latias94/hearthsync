use super::*;

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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        item.group == ApplyGroup::WtfCommon && item.target_account.as_deref() == Some("ACC_A")
    }));
}

#[test]
fn plan_bundle_apply_rejects_invalid_embedded_manifest() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("invalid-manifest-bundle.zip");
    let installation = create_fixture_installation(temp.path(), false);
    let mut manifest = sample_manifest();
    manifest.schema_version = 0;
    let manifest = toml::to_string_pretty(&manifest).expect("manifest");
    create_archive_with_raw_entries(
        &bundle_path,
        &[
            (MANIFEST_ENTRY, &manifest),
            ("addons/WeakAuras/WeakAuras.toc", "toc"),
        ],
    );

    let error = plan_bundle_apply(&bundle_path, &installation, &BundleApplyMappings::default())
        .expect_err("invalid manifest should fail before apply planning");

    assert!(
        error
            .to_string()
            .contains("schema_version must be greater than zero")
    );
}

#[test]
fn plan_bundle_apply_requires_explicit_common_account_selection_for_common_only_bundle() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.resources.wtf_characters.clear();

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ONLYACC")
            .join("SavedVariables"),
    )
    .expect("target account");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
    .expect_err("common-only bundle should require explicit account selection");

    assert!(
        error
            .to_string()
            .contains("common WTF resources require explicit target account selection")
    );
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrite_count").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrite_applied").is_none())
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        .filter(|operation| operation.action == ApplyAction::Add)
    {
        if groups.last().copied() != Some(operation.group) {
            groups.push(operation.group);
        }
    }

    assert_eq!(
        groups,
        vec![
            ApplyGroup::Addons,
            ApplyGroup::InterfaceAssets,
            ApplyGroup::Fonts,
            ApplyGroup::WtfCommon,
            ApplyGroup::WtfCharacters,
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
    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables"),
    )
    .expect("root saved variables");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
        "DetailsDB = {}",
    )
    .expect("root saved variable");
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
    assert!(
        archive
            .by_name("wtf/common/root/SavedVariables/RootDetails.lua")
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
        Some(WtfScope::GlobalConfig)
    );
    assert_eq!(
        scope_for("wtf/common/root/SavedVariables/RootDetails.lua"),
        Some(WtfScope::RootSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/account-settings.wtf"),
        Some(WtfScope::AccountRootFile)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
        Some(WtfScope::AccountSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua"),
        Some(WtfScope::CharacterSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt"),
        Some(WtfScope::CharacterState)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/config-cache.wtf"),
        Some(WtfScope::CacheLike)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/config-cache.wtf"),
        Some(WtfScope::CacheLike)
    );
}

#[test]
fn plan_bundle_apply_reports_existing_files_as_replace_in_logical_plan() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
    assert!(plan.summary.files_to_replace > 0);
    assert_eq!(plan.summary.files_to_replace, plan.operations.len());
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(
        plan.group_policies.addons.policy,
        ResourceApplyPolicy::Merge
    );
    assert!(
        plan.operations
            .iter()
            .all(|operation| operation.action == ApplyAction::Replace)
    );
}

#[test]
fn plan_apply_from_entries_with_reader_skips_byte_reads_for_deterministic_add_operations() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), false);
    let read_calls = Cell::new(0usize);

    let plan = plan_apply_from_entries_with_reader(
        std::path::Path::new("test.bundle.zip"),
        &installation,
        sample_manifest(),
        &["addons/WeakAuras/WeakAuras.toc".to_string()],
        &BundleApplyMappings::default(),
        |_archive_name| {
            read_calls.set(read_calls.get() + 1);
            Ok(b"unused".to_vec())
        },
    )
    .expect("plan apply from entries");

    assert_eq!(read_calls.get(), 0);
    assert_eq!(plan.summary.files_to_add, 1);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].action, ApplyAction::Add);
}

#[test]
fn plan_apply_from_entries_with_reader_keeps_existing_targets_logical_without_byte_reads() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let read_calls = Cell::new(0usize);

    let plan = plan_apply_from_entries_with_reader(
        std::path::Path::new("test.bundle.zip"),
        &installation,
        sample_manifest(),
        &["addons/WeakAuras/WeakAuras.toc".to_string()],
        &BundleApplyMappings::default(),
        |_archive_name| {
            read_calls.set(read_calls.get() + 1);
            Ok(b"## Interface: 110000".to_vec())
        },
    )
    .expect("plan apply from entries");

    assert_eq!(read_calls.get(), 0);
    assert_eq!(plan.summary.files_to_add, 0);
    assert_eq!(plan.summary.files_to_replace, 1);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.operations.len(), 1);
    assert_eq!(plan.operations[0].action, ApplyAction::Replace);
}

#[test]
fn prepare_bundle_apply_projects_preview_operations_into_execution_operations() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
        manifest: sample_manifest_with_rewrite(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let prepared = prepare_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("prepare bundle");

    let plan_projection = prepared
        .plan
        .operations
        .iter()
        .map(|operation| {
            (
                operation.action,
                operation.archive_name.clone(),
                operation.destination.clone(),
            )
        })
        .collect::<Vec<_>>();
    let execution_projection = prepared
        .execution_operations
        .iter()
        .map(|operation| {
            (
                operation.action,
                operation.archive_name.clone(),
                operation.destination.clone(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(execution_projection, plan_projection);
    assert!(
        prepared
            .execution_operations
            .iter()
            .any(|operation| !operation.rewrites.is_empty())
    );

    let serialized_operations = serde_json::to_value(&prepared.plan)
        .expect("serialize prepared plan")
        .get("operations")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("operations array");
    assert!(
        serialized_operations
            .iter()
            .all(|operation| operation.get("rewrites").is_none())
    );
}

#[test]
fn plan_bundle_apply_rejects_case_insensitive_target_collisions_on_macos() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("case-collision-bundle.zip");
    let installation =
        create_fixture_installation_on_platform(temp.path(), false, HostPlatform::MacOs);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries(
        &bundle_path,
        &[
            (MANIFEST_ENTRY, &manifest),
            ("addons/WeakAuras/WeakAuras.toc", "toc-a"),
            ("addons/weakauras/weakauras.toc", "toc-b"),
        ],
    );

    let error = plan_bundle_apply(&bundle_path, &installation, &BundleApplyMappings::default())
        .expect_err("case-insensitive target collisions should fail on macOS");

    let message = error.to_string();
    assert!(message.contains("case-insensitive target path collisions"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
    assert!(message.contains("addons/weakauras/weakauras.toc"));
}

#[test]
fn plan_bundle_apply_allows_case_distinct_targets_on_linux() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("case-distinct-bundle.zip");
    let installation =
        create_fixture_installation_on_platform(temp.path(), false, HostPlatform::Linux);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries(
        &bundle_path,
        &[
            (MANIFEST_ENTRY, &manifest),
            ("addons/WeakAuras/WeakAuras.toc", "toc-a"),
            ("addons/weakauras/weakauras.toc", "toc-b"),
        ],
    );

    let plan = plan_bundle_apply(&bundle_path, &installation, &BundleApplyMappings::default())
        .expect("case-distinct targets should be allowed on Linux");

    assert_eq!(plan.summary.files_to_add, 2);
}
