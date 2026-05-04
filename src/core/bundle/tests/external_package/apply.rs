use super::*;

#[test]
fn apply_external_package_keeps_case_mixed_addon_subtree_on_macos_target() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_root = source.path().join("AuthorUI");
    let exact_case_root = package_root
        .join("Interface")
        .join("AddOns")
        .join("WeakAuras");
    let mixed_case_root = package_root
        .join("Interface")
        .join("AddOns")
        .join("weakauras");

    fs::create_dir_all(&exact_case_root).expect("exact addon root");
    fs::create_dir_all(mixed_case_root.join("Modules")).expect("mixed-case addon subtree");
    fs::write(
        exact_case_root.join("WeakAuras.toc"),
        "## Interface: 110000\n## Title: WeakAuras\n",
    )
    .expect("toc");
    fs::write(mixed_case_root.join("Core.lua"), "print('wa core')").expect("core");
    fs::write(
        mixed_case_root.join("Modules").join("Module.lua"),
        "print('wa module')",
    )
    .expect("module");

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    let result = apply_external_package(ApplyExternalPackageRequest {
        external_package: CreateExternalPackageBundleRequest {
            source_path: package_root,
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
            apply_defaults: Some(ApplyDefaults {
                create_backup: false,
                addons: ResourceApplyPolicy::Mirror,
                wtf_common: ResourceApplyPolicy::Share,
                wtf_characters: ResourceApplyPolicy::ReplaceSelected,
                fonts: ResourceApplyPolicy::Mirror,
                interface_assets: ResourceApplyPolicy::Mirror,
            }),
        },
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: None,
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("apply external package with case-mixed addon subtree");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert!(!result.dry_run);
    assert_eq!(result.written_files, 3);
    assert_eq!(
        fs::read_to_string(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("Core.lua")
        )
        .expect("core"),
        "print('wa core')"
    );
    assert_eq!(
        fs::read_to_string(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("Modules")
                .join("Module.lua")
        )
        .expect("module"),
        "print('wa module')"
    );
}

#[test]
fn plan_external_package_apply_wraps_normalization_and_bundle_planning() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation = create_fixture_installation(target.path(), false);
    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: CreateExternalPackageBundleRequest {
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
        },
        installation: target_installation.clone(),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package apply");

    assert_eq!(
        plan.analysis.resources.addons,
        vec!["WeakAuras".to_string()]
    );
    assert_eq!(plan.target_flavor_root, target_installation.flavor_root);
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
}

#[test]
fn plan_external_package_apply_uses_author_package_default_profile_when_apply_defaults_missing() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: sample_external_package_request_with_apply_defaults(package_path, None),
        installation: target_installation.clone(),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package apply with default profile");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert!(plan.manifest.apply.create_backup);
    assert_eq!(
        plan.group_policies.addons.policy,
        ResourceApplyPolicy::Mirror
    );
    assert_eq!(
        plan.group_policies.wtf_common.policy,
        ResourceApplyPolicy::Share
    );
    assert_eq!(
        plan.group_policies.wtf_characters.policy,
        ResourceApplyPolicy::ReplaceSelected
    );
    assert_eq!(
        plan.group_policies.fonts.policy,
        ResourceApplyPolicy::Mirror
    );
    assert_eq!(
        plan.group_policies.interface_assets.policy,
        ResourceApplyPolicy::Mirror
    );
    assert_eq!(plan.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(plan.summary.paths_to_remove, 4);
    assert_eq!(plan.summary.files_to_add, 8);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.summary.files_to_preserve, 1);

    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.fonts_dir
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.interface_dir.join("SharedXML")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination
                == target_installation
                    .wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("Illidan")
                    .join("Examplemage")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/Config.wtf"
            && operation.action == ApplyAction::Preserve
    }));
}

#[test]
fn external_package_apply_plan_does_not_expose_execution_only_fields() {
    let package_root = external_package_fixture_root();
    let target = tempdir().expect("target temp dir");
    let installation = create_fixture_installation(target.path(), false);

    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: sample_external_package_request_with_apply_defaults(package_root, None),
        installation,
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package");

    let serialized_plan = serde_json::to_value(&plan).expect("serialize external package plan");
    let operations = serialized_plan
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
    assert!(serialized_plan.get("prepared_apply").is_none());
    assert!(serialized_plan.get("apply_source").is_none());
    assert!(serialized_plan.get("entry_source_map").is_none());
}

#[test]
fn plan_external_package_apply_supports_windows_package_to_macos_target_with_policy_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: sample_external_package_request_with_apply_defaults(
            package_path,
            Some(ApplyDefaults {
                create_backup: false,
                addons: ResourceApplyPolicy::Mirror,
                wtf_common: ResourceApplyPolicy::Share,
                wtf_characters: ResourceApplyPolicy::ReplaceSelected,
                fonts: ResourceApplyPolicy::Preserve,
                interface_assets: ResourceApplyPolicy::ReplaceSelected,
            }),
        ),
        installation: target_installation.clone(),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package apply");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert_eq!(plan.manifest.source.platform, Some(HostPlatform::Windows));
    assert!(plan.manifest.mapping.allow_cross_platform);
    assert!(!plan.manifest.apply.create_backup);
    assert_eq!(
        plan.group_policies.addons.policy,
        ResourceApplyPolicy::Mirror
    );
    assert_eq!(
        plan.group_policies.wtf_common.policy,
        ResourceApplyPolicy::Share
    );
    assert_eq!(
        plan.group_policies.wtf_characters.policy,
        ResourceApplyPolicy::ReplaceSelected
    );
    assert_eq!(
        plan.group_policies.fonts.policy,
        ResourceApplyPolicy::Preserve
    );
    assert_eq!(
        plan.group_policies.interface_assets.policy,
        ResourceApplyPolicy::ReplaceSelected
    );
    assert_eq!(plan.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(plan.summary.paths_to_remove, 3);
    assert_eq!(plan.summary.files_to_add, 7);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.summary.files_to_preserve, 2);

    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.interface_dir.join("SharedXML")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination
                == target_installation
                    .wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("Illidan")
                    .join("Examplemage")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/Config.wtf"
            && operation.action == ApplyAction::Preserve
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "fonts/FRIZQT__.ttf" && operation.action == ApplyAction::Preserve
    }));
}

#[test]
fn apply_external_package_respects_policy_overrides_on_macos_target() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let result = apply_external_package(ApplyExternalPackageRequest {
        external_package: sample_external_package_request_with_apply_defaults(
            package_path,
            Some(ApplyDefaults {
                create_backup: false,
                addons: ResourceApplyPolicy::Mirror,
                wtf_common: ResourceApplyPolicy::Share,
                wtf_characters: ResourceApplyPolicy::ReplaceSelected,
                fonts: ResourceApplyPolicy::Preserve,
                interface_assets: ResourceApplyPolicy::ReplaceSelected,
            }),
        ),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("apply external package");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert!(!result.dry_run);
    assert_eq!(result.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(result.written_files, 7);
    assert_eq!(result.rewritten_files, 0);
    assert!(result.backup_path.is_none());
    assert_eq!(result.plan_summary.paths_to_remove, 3);
    assert_eq!(result.plan_summary.files_to_add, 7);
    assert_eq!(result.plan_summary.files_to_preserve, 2);

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
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("bindings-cache.wtf")
            .exists()
    );
    assert!(
        !target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("StaleCharacter.txt")
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
    assert_eq!(
        fs::read_to_string(target_installation.fonts_dir.join("FRIZQT__.ttf")).expect("font"),
        "mac-font"
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("old.blp")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(
            target_installation
                .interface_dir
                .join("SharedXML")
                .join("texture.blp")
        )
        .expect("texture")
        .trim_end(),
        "fixture-texture"
    );
}

#[test]
fn apply_external_package_applies_complex_windows_author_zip_to_macos_target() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("complex-author-ui-pack.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            (
                "AuthorUI/Interface/AddOns/WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Title: WeakAuras\n",
            ),
            (
                "AuthorUI/INTERFACE/ADDONS/weakauras/Core.lua",
                "print('core')",
            ),
            (
                "AuthorUI/INTERFACE/ADDONS/weakauras/Modules/Module.lua",
                "print('module')",
            ),
            (
                "__MACOSX/AuthorUI/Interface/AddOns/WeakAuras/._WeakAuras.toc",
                "resource fork",
            ),
            (
                "AuthorUI/INTERFACE/ADDONS/weakauras/._Core.lua",
                "resource fork",
            ),
            ("AuthorUI/Interface/AddOns/WeakAuras/.DS_Store", "noise"),
            ("AuthorUI/WTF/Config.wtf", "SET locale enUS"),
            (
                "AuthorUI/WTF/Account/ACCOUNT/SavedVariables/Details.lua",
                "DetailsDB = {}",
            ),
            (
                "AuthorUI/WTF/Account/ACCOUNT/bindings-cache.wtf",
                "target bindings",
            ),
            (
                "AuthorUI/WTF/Account/ACCOUNT/Illidan/Examplemage/AddOns.txt",
                "WeakAuras: enabled",
            ),
            (
                "AuthorUI/WTF/Account/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua",
                "PawnOptions = {}",
            ),
            ("AuthorUI/Fonts/FRIZQT__.ttf", "complex-font"),
            ("AuthorUI/Fonts/._FRIZQT__.ttf", "resource fork"),
            (
                "AuthorUI/INTERFACE/SharedXML/texture.blp",
                "complex-texture",
            ),
            ("AuthorUI/INTERFACE/SharedXML/Thumbs.db", "noise"),
        ],
    );

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let result = apply_external_package(ApplyExternalPackageRequest {
        external_package: sample_external_package_request_with_apply_defaults(package_path, None),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("apply complex external package");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert_eq!(result.manifest.source.platform, Some(HostPlatform::Windows));
    assert!(result.manifest.apply.create_backup);
    assert!(result.backup_path.is_some());
    assert_eq!(result.analysis.summary.total_files, 10);
    assert_eq!(result.analysis.summary.normalized_files, 10);
    assert_eq!(result.analysis.summary.warning_count, 0);
    assert_eq!(
        result.analysis.resources.addons,
        vec!["WeakAuras".to_string()]
    );
    assert_eq!(
        result.analysis.resources.interface_assets,
        vec!["SharedXML".to_string()]
    );
    assert_eq!(result.written_files, 9);
    assert_eq!(result.plan_summary.paths_to_remove, 4);
    assert_eq!(result.plan_summary.files_to_preserve, 1);

    let addon_root = target_installation.addon_dir.join("WeakAuras");
    assert!(addon_root.join("WeakAuras.toc").exists());
    assert_eq!(
        fs::read_to_string(addon_root.join("Core.lua")).expect("core"),
        "print('core')"
    );
    assert_eq!(
        fs::read_to_string(addon_root.join("Modules").join("Module.lua")).expect("module"),
        "print('module')"
    );
    assert!(!addon_root.join("Stale.lua").exists());
    assert!(!addon_root.join("._Core.lua").exists());
    assert!(!addon_root.join(".DS_Store").exists());

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
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("bindings-cache.wtf")
            .exists()
    );
    assert!(
        !target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("StaleCharacter.txt")
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
    assert_eq!(
        fs::read_to_string(target_installation.fonts_dir.join("FRIZQT__.ttf")).expect("font"),
        "complex-font"
    );
    assert!(
        !target_installation
            .fonts_dir
            .join("._FRIZQT__.ttf")
            .exists()
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("old.blp")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(
            target_installation
                .interface_dir
                .join("SharedXML")
                .join("texture.blp")
        )
        .expect("texture"),
        "complex-texture"
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("Thumbs.db")
            .exists()
    );
}
