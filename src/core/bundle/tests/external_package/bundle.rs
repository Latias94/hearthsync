use super::*;

#[test]
fn create_external_package_bundle_produces_reusable_first_party_bundle() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let prepared = create_external_package_bundle(CreateExternalPackageBundleRequest {
        source_path: package_path,
        layout: ExternalPackageLayout::Auto,
        source_account: None,
        source_server: None,
        source_character: None,
        source_flavor: WowFlavor::Retail,
        source_platform: Some(HostPlatform::Windows),
        supported_targets: vec![WowFlavor::Retail],
        output_path: None,
        package_id: None,
        package_name: None,
        created_by: None,
        description: None,
        apply_defaults: None,
    })
    .expect("create external package bundle");

    assert!(prepared.bundle.archive_path.exists());
    assert_eq!(prepared.manifest.package.id, "author-ui-pack");
    assert_eq!(
        prepared.manifest.resources.addons,
        vec!["WeakAuras".to_string()]
    );
    assert_eq!(
        prepared.manifest.mapping.character_mode,
        CharacterMappingMode::Prompt
    );

    let inspection = inspect_bundle(&prepared.bundle.archive_path).expect("inspect bundle");
    assert_eq!(inspection.manifest.package.id, "author-ui-pack");
    assert_eq!(inspection.entries.addons, 2);
    assert_eq!(inspection.entries.wtf_common, 3);
    assert_eq!(inspection.entries.wtf_characters, 2);
    assert_eq!(inspection.entries.fonts, 1);
    assert_eq!(inspection.entries.interface_assets, 1);
}

#[test]
fn external_package_bundle_can_reuse_plan_and_unpack_pipeline() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let prepared = create_external_package_bundle(CreateExternalPackageBundleRequest {
        source_path: package_path,
        layout: ExternalPackageLayout::Auto,
        source_account: None,
        source_server: None,
        source_character: None,
        source_flavor: WowFlavor::Retail,
        source_platform: Some(HostPlatform::Windows),
        supported_targets: vec![WowFlavor::Retail],
        output_path: None,
        package_id: Some("author-ui-import".to_string()),
        package_name: Some("Author UI Import".to_string()),
        created_by: Some("hearthsync-test".to_string()),
        description: None,
        apply_defaults: None,
    })
    .expect("create external package bundle");
    let target_installation = create_fixture_installation(target.path(), false);
    let apply_mappings = BundleApplyMappings {
        target_account: Some("ACCOUNT".to_string()),
        target_server: Some("Illidan".to_string()),
        target_character: Some("Examplemage".to_string()),
        ..BundleApplyMappings::default()
    };

    let plan = plan_bundle_apply(
        &prepared.bundle.archive_path,
        &target_installation,
        &apply_mappings,
    )
    .expect("plan external package bundle");
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == ApplyGroup::Addons)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == ApplyGroup::WtfCommon)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == ApplyGroup::WtfCharacters)
    );

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path: prepared.bundle.archive_path.clone(),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings,
    })
    .expect("apply external package bundle");

    assert!(result.written_files > 0);
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
            .join("SavedVariables")
            .join("Pawn.lua")
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
}
