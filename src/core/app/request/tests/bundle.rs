use super::*;

#[test]
fn apply_bundle_request_converts_app_owned_apply_mappings() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

    let domain: DomainUnpackBundleRequest = ApplyBundleAppRequest {
        bundle_path: PathBuf::from("bundle.zip"),
        installation: sample_installation(),
        dry_run: true,
        backup_output_path: Some(PathBuf::from("backup")),
        apply_mappings: BundleApplyMappingsValue {
            target_account: Some("AccountA".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Main".to_string()),
            selected_accounts: vec!["AccountA".to_string()],
            all_accounts: true,
            characters: vec![BundleCharacterMappingOverrideValue {
                source_account: Some("SourceAccount".to_string()),
                source_server: "Stormrage".to_string(),
                source_character: "SourceMain".to_string(),
                target_account: Some("TargetAccount".to_string()),
                target_server: "Illidan".to_string(),
                target_character: "TargetMain".to_string(),
            }],
        },
    }
    .into_domain_request(&runtime)
    .expect("apply bundle request");

    assert_eq!(domain.bundle_path, base.join("bundle.zip"));
    assert_eq!(domain.backup_output_path, Some(base.join("backup")));
    assert!(domain.dry_run);
    assert_eq!(
        domain.apply_mappings.target_account.as_deref(),
        Some("AccountA")
    );
    assert_eq!(
        domain.apply_mappings.target_server.as_deref(),
        Some("Illidan")
    );
    assert_eq!(
        domain.apply_mappings.target_character.as_deref(),
        Some("Main")
    );
    assert_eq!(domain.apply_mappings.selected_accounts, vec!["AccountA"]);
    assert!(domain.apply_mappings.all_accounts);
    assert_eq!(domain.apply_mappings.characters.len(), 1);
    assert_eq!(
        domain.apply_mappings.characters[0]
            .source_account
            .as_deref(),
        Some("SourceAccount")
    );
}

#[test]
fn apply_bundle_request_rejects_invalid_app_apply_mappings() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base);

    let error = ApplyBundleAppRequest {
        bundle_path: PathBuf::from("bundle.zip"),
        installation: sample_installation(),
        dry_run: true,
        backup_output_path: None,
        apply_mappings: BundleApplyMappingsValue {
            target_account: Some("Invalid*Account".to_string()),
            ..BundleApplyMappingsValue::default()
        },
    }
    .into_domain_request(&runtime)
    .expect_err("invalid app apply mappings should fail closed");

    assert!(error.to_string().contains("invalid target account name"));
}

#[test]
fn pack_bundle_request_converts_app_owned_manifest() {
    let base = std::env::current_dir().expect("cwd");
    let runtime = runtime_with_relative_path_base(base.clone());

    let domain: DomainPackBundleRequest = PackBundleAppRequest {
        installation: sample_installation(),
        manifest: sample_manifest(),
        output_path: Some(PathBuf::from("bundle.zip")),
        manifest_base_dir: Some(PathBuf::from("manifest-dir")),
    }
    .into_domain_request(&runtime)
    .expect("pack bundle request");

    assert_eq!(domain.manifest_base_dir, Some(base.join("manifest-dir")));
    assert_eq!(
        domain.output_path,
        Some(base.join("manifest-dir").join("bundle.zip"))
    );
    assert_eq!(domain.manifest.schema_version, 1);
    assert_eq!(domain.manifest.package.id, "author-ui");
    assert_eq!(
        domain.manifest.source.flavor,
        crate::core::install::WowFlavor::Retail
    );
    assert_eq!(domain.manifest.resources.addons, vec!["WeakAuras"]);
    assert_eq!(domain.manifest.resources.wtf_characters.len(), 1);
    assert_eq!(
        domain.manifest.mapping.character_mode,
        CharacterMappingMode::Explicit
    );
    assert_eq!(domain.manifest.apply.addons, ResourceApplyPolicy::Mirror);
}

#[test]
fn pack_bundle_request_rejects_invalid_app_manifest_before_core_projection() {
    let runtime = AppRuntime::new();
    let mut manifest = sample_manifest();
    manifest.schema_version = 0;

    let error = PackBundleAppRequest {
        installation: sample_installation(),
        manifest,
        output_path: None,
        manifest_base_dir: None,
    }
    .into_domain_request(&runtime)
    .expect_err("invalid manifest should fail before core packing");

    assert!(
        error
            .to_string()
            .contains("schema_version must be greater than zero")
    );
}

#[test]
fn pack_bundle_request_resolves_relative_output_without_manifest_base_against_installation_parent()
{
    let runtime = AppRuntime::new();
    let installation = sample_installation();
    let expected_output_path = installation
        .product_root
        .parent()
        .expect("installation parent")
        .join("exports");

    let domain: DomainPackBundleRequest = PackBundleAppRequest {
        installation,
        manifest: sample_manifest(),
        output_path: Some(PathBuf::from("exports")),
        manifest_base_dir: None,
    }
    .into_domain_request(&runtime)
    .expect("pack bundle request");

    assert_eq!(domain.output_path, Some(expected_output_path));
}
