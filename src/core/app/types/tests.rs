use std::path::PathBuf;

use crate::core::app::{
    AddonPackageMetadataValue, AddonProviderOptionsValue, AddonProviderRetryPolicyValue,
    BundleApplyDefaultsValue, BundleApplyMappingsValue, BundleCharacterMappingOverrideValue,
    BundleCharacterResourceValue, BundleManifestValue, BundleMappingRulesValue,
    BundlePackageValue, BundleResourcesValue, BundleSourceValue, CharacterMappingModeValue,
    HealthStatusValue, HostPlatformValue, ResourceApplyPolicyValue, WowFlavorValue,
};
use crate::core::manifest::ResourceApplyPolicy;

#[test]
fn host_platform_value_roundtrips_domain_shape() {
    let value = HostPlatformValue::MacOs;

    let domain = value.into_domain();

    assert_eq!(HostPlatformValue::from_domain(domain), value);
}

#[test]
fn wow_flavor_value_roundtrips_domain_shape() {
    let value = WowFlavorValue::ClassicEra;

    let domain = value.into_domain();

    assert_eq!(WowFlavorValue::from_domain(domain), value);
}

#[test]
fn wow_flavor_value_helpers_return_stable_strings() {
    assert_eq!(WowFlavorValue::Retail.as_str(), "retail");
    assert_eq!(WowFlavorValue::ClassicEra.as_str(), "classic_era");
}

#[test]
fn character_mapping_mode_value_roundtrips_domain_shape() {
    let value = CharacterMappingModeValue::Prompt;

    let domain = value.into_domain();

    assert_eq!(CharacterMappingModeValue::from_domain(domain), value);
}

#[test]
fn health_status_value_roundtrips_domain_shape() {
    let value = HealthStatusValue::Warning;

    let domain = value.into_domain();

    assert_eq!(HealthStatusValue::from_domain(domain), value);
}

#[test]
fn addon_provider_retry_policy_value_roundtrips_domain_shape() {
    let value = AddonProviderRetryPolicyValue { max_attempts: 3 };

    let domain = value.clone().into_domain();

    assert_eq!(AddonProviderRetryPolicyValue::from_domain(domain), value);
}

#[test]
fn addon_provider_options_value_roundtrips_domain_shape() {
    let value = AddonProviderOptionsValue {
        download_cache_dir: Some(PathBuf::from("cache")),
        retry_policy: AddonProviderRetryPolicyValue { max_attempts: 2 },
    };

    let domain = value.clone().into_domain();

    assert_eq!(AddonProviderOptionsValue::from_domain(domain), value);
}

#[test]
fn addon_package_metadata_value_roundtrips_domain_shape() {
    let value = AddonPackageMetadataValue {
        index_name: Some("curated".to_string()),
        index_package_id: Some("weakauras".to_string()),
        package_name: Some("WeakAuras".to_string()),
        version: Some("1.2.3".to_string()),
        source_url: Some("https://example.invalid/weakauras.zip".to_string()),
        website_url: Some("https://example.invalid/weakauras".to_string()),
        source_sha256: Some("abc123".to_string()),
        supported_flavors: vec!["retail".to_string(), "classic".to_string()],
    };

    let domain = value.clone().into_domain();

    assert_eq!(AddonPackageMetadataValue::from_domain(domain), value);
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

    let domain = value.clone().into_domain();

    assert_eq!(BundleApplyMappingsValue::from_domain(domain), value);
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
    let value = BundleManifestValue {
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
            interface_assets: vec!["Interface/Buttons".to_string()],
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
    };

    let domain = value.clone().into_domain();

    assert_eq!(BundleManifestValue::from_domain(domain), value);
}
