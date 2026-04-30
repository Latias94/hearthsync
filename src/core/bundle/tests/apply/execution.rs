use super::*;

#[test]
fn unpack_bundle_restores_files_and_creates_backup() {
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
fn unpack_bundle_dry_run_still_skips_identical_files_after_prepare() {
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

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation,
        dry_run: true,
        backup_output_path: None,
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("dry-run identical bundle");

    assert!(result.dry_run);
    assert_eq!(result.plan_summary.files_to_add, 0);
    assert_eq!(result.plan_summary.files_to_replace, 0);
    assert!(result.plan_summary.files_to_skip > 0);
    assert_eq!(result.plan_summary.files_to_skip, result.planned_files);
    assert_eq!(result.written_files, 0);
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
    .expect("root saved variable");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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

    let root_common_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
    )
    .expect("root common lua");
    assert!(root_common_lua.contains("Targetmage - Stormrage"));
    assert!(root_common_lua.contains("Default.Stormrage.Targetmage"));

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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
fn unpack_bundle_rejects_symlink_payload_entries() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("symlink-payload-bundle.zip");
    let installation = create_fixture_installation(temp.path(), false);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries_and_symlink(
        &bundle_path,
        &[(MANIFEST_ENTRY, &manifest)],
        "addons/WeakAuras/WeakAuras.toc",
        "../outside.toc",
    );

    let error = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect_err("symlink payload should fail");

    let message = error.to_string();
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
}

#[test]
fn unpack_bundle_rejects_non_portable_entry_paths() {
    let temp = tempdir().expect("temp dir");
    let bundle_path = temp.path().join("unsafe-path-bundle.zip");
    let installation = create_fixture_installation(temp.path(), false);
    let manifest = toml::to_string_pretty(&sample_manifest()).expect("manifest");
    create_archive_with_raw_entries(
        &bundle_path,
        &[
            (MANIFEST_ENTRY, &manifest),
            ("addons/Weak:Auras/WeakAuras.toc", "toc"),
        ],
    );

    let error = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect_err("unsafe bundle entry path should fail");

    assert!(error.to_string().contains("unsafe archive path"));
}
