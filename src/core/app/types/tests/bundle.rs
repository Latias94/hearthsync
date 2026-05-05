use crate::core::app::{
    BundleApplyDefaultsValue, BundleApplyMappingsValue, BundleCharacterMappingOverrideValue,
    BundleCharacterResourceValue, BundleManifestValue, BundleMappingRulesValue, BundlePackageValue,
    BundleResourcesValue, BundleSourceValue, CharacterMappingModeValue, HostPlatformValue,
    ResourceApplyPolicyValue, WowFlavorValue, WtfScopeRiskValue,
};
use crate::core::bundle::{WtfScope, WtfScopeRisk};
use crate::core::manifest::ResourceApplyPolicy;

#[test]
fn character_mapping_mode_value_roundtrips_domain_shape() {
    let value = CharacterMappingModeValue::Prompt;

    let domain = value.into_domain();

    assert_eq!(CharacterMappingModeValue::from_domain(domain), value);
}

#[test]
fn bundle_apply_mappings_value_roundtrips_domain_shape() {
    let value = BundleApplyMappingsValue {
        target_account: Some("AccountA".to_string()),
        target_server: Some("Illidan".to_string()),
        target_character: Some("Main".to_string()),
        selected_accounts: vec!["AccountA".to_string(), "AccountB".to_string()],
        all_accounts: true,
        characters: vec![BundleCharacterMappingOverrideValue {
            source_account: Some("SourceAccount".to_string()),
            source_server: "Stormrage".to_string(),
            source_character: "SourceMain".to_string(),
            target_account: Some("TargetAccount".to_string()),
            target_server: "Illidan".to_string(),
            target_character: "TargetMain".to_string(),
        }],
    };

    let domain = value.clone().into_domain().expect("apply mappings");

    assert_eq!(BundleApplyMappingsValue::from_domain(domain), value);
}

#[test]
fn bundle_apply_mappings_value_rejects_invalid_target_identity() {
    let error = BundleApplyMappingsValue {
        target_account: Some("AccountA ".to_string()),
        ..BundleApplyMappingsValue::default()
    }
    .into_domain()
    .expect_err("invalid target account should fail closed");

    assert!(error.to_string().contains("invalid target account name"));
}

#[test]
fn bundle_apply_mappings_value_rejects_invalid_selected_account() {
    let error = BundleApplyMappingsValue {
        selected_accounts: vec!["CON".to_string()],
        ..BundleApplyMappingsValue::default()
    }
    .into_domain()
    .expect_err("invalid selected account should fail closed");

    assert!(error.to_string().contains("invalid selected account name"));
}

#[test]
fn bundle_apply_mappings_value_rejects_invalid_character_override() {
    let error = BundleApplyMappingsValue {
        characters: vec![BundleCharacterMappingOverrideValue {
            source_account: Some("SourceAccount".to_string()),
            source_server: "Stormrage".to_string(),
            source_character: "SourceMain".to_string(),
            target_account: Some("TargetAccount".to_string()),
            target_server: "Illidan".to_string(),
            target_character: "Target*Main".to_string(),
        }],
        ..BundleApplyMappingsValue::default()
    }
    .into_domain()
    .expect_err("invalid character override should fail closed");

    assert!(error.to_string().contains("invalid target character name"));
}

#[test]
fn bundle_apply_mappings_value_rejects_overlapping_character_overrides() {
    let error = BundleApplyMappingsValue {
        characters: vec![
            BundleCharacterMappingOverrideValue {
                source_account: None,
                source_server: "Stormrage".to_string(),
                source_character: "SourceMain".to_string(),
                target_account: Some("TargetAccount".to_string()),
                target_server: "Illidan".to_string(),
                target_character: "TargetMain".to_string(),
            },
            BundleCharacterMappingOverrideValue {
                source_account: Some("SourceAccount".to_string()),
                source_server: "stormrage".to_string(),
                source_character: "SourceMain".to_string(),
                target_account: Some("OtherTargetAccount".to_string()),
                target_server: "Illidan".to_string(),
                target_character: "OtherTargetMain".to_string(),
            },
        ],
        ..BundleApplyMappingsValue::default()
    }
    .into_domain()
    .expect_err("overlapping character overrides should fail closed");

    assert!(
        error
            .to_string()
            .contains("overlapping character mapping override")
    );
}

#[test]
fn bundle_apply_defaults_value_author_package_defaults_match_shared_profile() {
    let defaults = BundleApplyDefaultsValue::author_package_defaults();

    assert!(defaults.create_backup);
    assert_eq!(defaults.addons, ResourceApplyPolicyValue::Mirror);
    assert_eq!(defaults.wtf_common, ResourceApplyPolicyValue::Share);
    assert_eq!(
        defaults.wtf_characters,
        ResourceApplyPolicyValue::ReplaceSelected
    );
    assert_eq!(defaults.fonts, ResourceApplyPolicyValue::Mirror);
    assert_eq!(defaults.interface_assets, ResourceApplyPolicyValue::Mirror);
}

#[test]
fn wtf_scope_risk_value_projects_domain_risk() {
    assert_eq!(WtfScope::AccountSavedVariables.risk(), WtfScopeRisk::High);
    assert_eq!(
        WtfScopeRiskValue::from_domain(WtfScope::CacheLike.risk()),
        WtfScopeRiskValue::Low
    );
}

#[test]
fn bundle_apply_defaults_value_converts_back_to_domain_defaults() {
    let value = BundleApplyDefaultsValue {
        create_backup: false,
        addons: ResourceApplyPolicyValue::Merge,
        wtf_common: ResourceApplyPolicyValue::Share,
        wtf_characters: ResourceApplyPolicyValue::Sync,
        fonts: ResourceApplyPolicyValue::Preserve,
        interface_assets: ResourceApplyPolicyValue::ReplaceSelected,
    };

    let domain = value.clone().into_domain();

    assert!(!domain.create_backup);
    assert_eq!(domain.addons, ResourceApplyPolicy::Merge);
    assert_eq!(domain.wtf_common, ResourceApplyPolicy::Share);
    assert_eq!(domain.wtf_characters, ResourceApplyPolicy::Sync);
    assert_eq!(domain.fonts, ResourceApplyPolicy::Preserve);
    assert_eq!(
        domain.interface_assets,
        ResourceApplyPolicy::ReplaceSelected
    );
    assert_eq!(BundleApplyDefaultsValue::from_domain(domain), value);
}

#[test]
fn bundle_manifest_value_roundtrips_domain_shape() {
    let value = valid_bundle_manifest_value();

    let domain = value.clone().into_domain().expect("bundle manifest");

    assert_eq!(BundleManifestValue::from_domain(domain), value);
}

#[test]
fn bundle_manifest_value_rejects_invalid_manifest() {
    let mut value = valid_bundle_manifest_value();
    value.schema_version = 0;

    let error = value
        .into_domain()
        .expect_err("invalid manifest should fail closed");

    assert!(
        error
            .to_string()
            .contains("schema_version must be greater than zero")
    );
}

fn valid_bundle_manifest_value() -> BundleManifestValue {
    BundleManifestValue {
        schema_version: 1,
        package: BundlePackageValue {
            id: "author-ui".to_string(),
            name: "Author UI".to_string(),
            created_by: "tester".to_string(),
            description: Some("fixture manifest".to_string()),
        },
        source: BundleSourceValue {
            flavor: WowFlavorValue::Retail,
            platform: Some(HostPlatformValue::Windows),
            exported_at: Some("2026-04-18T10:00:00Z".to_string()),
            supported_targets: vec![WowFlavorValue::Retail, WowFlavorValue::Classic],
        },
        resources: BundleResourcesValue {
            addons: vec!["WeakAuras".to_string()],
            wtf_common: true,
            wtf_characters: vec![BundleCharacterResourceValue {
                source_account: Some("AccountA".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Main".to_string(),
                target_hint: Some("Main".to_string()),
            }],
            fonts: true,
            interface_assets: vec!["Buttons".to_string()],
            addon_lock: true,
            addon_indexes: vec!["metadata/addons/index.toml".to_string()],
        },
        mapping: BundleMappingRulesValue {
            character_mode: CharacterMappingModeValue::Explicit,
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
            allow_cross_platform: true,
        },
        apply: BundleApplyDefaultsValue {
            create_backup: true,
            addons: ResourceApplyPolicyValue::Mirror,
            wtf_common: ResourceApplyPolicyValue::Share,
            wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
            fonts: ResourceApplyPolicyValue::Mirror,
            interface_assets: ResourceApplyPolicyValue::Mirror,
        },
    }
}
