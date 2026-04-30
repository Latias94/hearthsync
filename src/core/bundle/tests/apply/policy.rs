use super::*;

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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
            .all(|operation| operation.action == ApplyAction::Preserve)
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
            && operation.action == ApplyAction::Preserve
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"
            && operation.action == ApplyAction::Add
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        operation.action == ApplyAction::Remove
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
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
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
        operation.action == ApplyAction::Remove
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
