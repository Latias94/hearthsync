use std::path::PathBuf;

use crate::core::addon::AddonStateStorageKind;
use crate::core::addon::policy::{AddonPolicyPin, AddonReleaseChannel};
use crate::core::addon::{
    AddonDependencyResolutionCapability, AddonDependencyResolutionStrategy, AddonStatePaths,
};
use crate::core::app::{
    AddonDependencyResolutionCapabilityValue, AddonDependencyResolutionStrategyValue,
    AddonManagementCapabilitiesValue, AddonPackageMetadataValue, AddonPolicyPinValue,
    AddonProviderOptionsValue, AddonProviderRetryPolicyValue, AddonReleaseChannelValue,
    AddonStatePathsValue, AddonStateStorageValue, BundleApplyDefaultsValue,
    BundleApplyMappingsValue, BundleCharacterMappingOverrideValue, BundleCharacterResourceValue,
    BundleManifestValue, BundleMappingRulesValue, BundlePackageValue, BundleResourcesValue,
    BundleSourceValue, CharacterMappingModeValue, HealthStatusValue, HostPlatformValue,
    HttpNoValidatorCachePolicyValue, ResolvedInstallationValue, ResourceApplyPolicyValue,
    WowFlavorValue,
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
fn resolved_installation_value_projects_absolute_domain_shape() {
    let value = absolute_installation_value();

    let domain = value.clone().into_domain().expect("domain installation");

    assert_eq!(domain.product_root, value.product_root);
    assert_eq!(domain.flavor_root, value.flavor_root);
    assert_eq!(domain.interface_dir, value.interface_dir);
    assert_eq!(domain.addon_dir, value.addon_dir);
    assert_eq!(domain.wtf_dir, value.wtf_dir);
    assert_eq!(domain.fonts_dir, value.fonts_dir);
}

#[test]
fn resolved_installation_value_rejects_relative_paths() {
    let error = relative_installation_value()
        .into_domain()
        .expect_err("relative installation should fail closed");

    assert!(
        error
            .to_string()
            .contains("resolved installation product root must be absolute")
    );
}

#[test]
fn addon_provider_retry_policy_value_roundtrips_domain_shape() {
    let value = AddonProviderRetryPolicyValue { max_attempts: 3 };

    let domain = value.clone().into_domain().expect("retry policy");

    assert_eq!(AddonProviderRetryPolicyValue::from_domain(domain), value);
}

#[test]
fn addon_provider_retry_policy_value_rejects_zero_attempts() {
    let error = AddonProviderRetryPolicyValue { max_attempts: 0 }
        .into_domain()
        .expect_err("zero retry attempts should fail closed");

    assert!(
        error
            .to_string()
            .contains("addon provider retry policy max_attempts must be greater than zero")
    );
}

#[test]
fn addon_provider_options_value_roundtrips_domain_shape() {
    let value = AddonProviderOptionsValue {
        download_cache_dir: Some(PathBuf::from("cache")),
        retry_policy: AddonProviderRetryPolicyValue { max_attempts: 2 },
        http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue::ReuseWithinWindow {
            max_age_secs: 120,
        },
    };

    let domain = value.clone().into_domain().expect("provider options");

    assert_eq!(AddonProviderOptionsValue::from_domain(domain), value);
}

#[test]
fn addon_provider_options_value_rejects_invalid_retry_policy() {
    let error = AddonProviderOptionsValue {
        retry_policy: AddonProviderRetryPolicyValue { max_attempts: 0 },
        ..AddonProviderOptionsValue::default()
    }
    .into_domain()
    .expect_err("invalid retry policy should fail closed");

    assert!(
        error
            .to_string()
            .contains("addon provider retry policy max_attempts must be greater than zero")
    );
}

#[test]
fn http_no_validator_cache_policy_value_roundtrips_domain_shape() {
    let value = HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 120 };

    let domain = value.clone().into_domain().expect("cache policy");

    assert_eq!(HttpNoValidatorCachePolicyValue::from_domain(domain), value);
}

#[test]
fn http_no_validator_cache_policy_value_rejects_zero_window() {
    let error = HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 0 }
        .into_domain()
        .expect_err("zero cache window should fail closed");

    assert!(
        error
            .to_string()
            .contains("HTTP no-validator cache window must be greater than zero seconds")
    );
}

#[test]
fn addon_provider_options_value_rejects_invalid_no_validator_cache_policy() {
    let error = AddonProviderOptionsValue {
        http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue::ReuseWithinWindow {
            max_age_secs: 0,
        },
        ..AddonProviderOptionsValue::default()
    }
    .into_domain()
    .expect_err("invalid no-validator cache policy should fail closed");

    assert!(
        error
            .to_string()
            .contains("HTTP no-validator cache window must be greater than zero seconds")
    );
}

#[test]
fn addon_state_storage_value_roundtrips_domain_shape() {
    let value = AddonStateStorageValue::Sidecar;

    let domain = value.into_domain();

    assert_eq!(domain, AddonStateStorageKind::Sidecar);
    assert_eq!(AddonStateStorageValue::from_domain(domain), value);
}

#[test]
fn addon_state_paths_value_projects_domain_shape() {
    let domain = AddonStatePaths {
        root_dir: PathBuf::from("state"),
        registry_path: PathBuf::from("state/addons.toml"),
        lock_path: PathBuf::from("state/lock.toml"),
        policy_path: PathBuf::from("state/addon-policy.toml"),
        adopted_dir: PathBuf::from("state/adopted"),
    };

    assert_eq!(
        AddonStatePathsValue::from_domain(domain),
        AddonStatePathsValue {
            root_dir: PathBuf::from("state"),
            registry_path: PathBuf::from("state/addons.toml"),
            lock_path: PathBuf::from("state/lock.toml"),
            policy_path: PathBuf::from("state/addon-policy.toml"),
            adopted_dir: PathBuf::from("state/adopted"),
        }
    );
}

#[test]
fn addon_management_capabilities_value_exposes_scan_only_and_managed_contract() {
    let value = AddonManagementCapabilitiesValue {
        state_storage: AddonStateStorageValue::AppData,
        scan_only_without_managed_state: true,
        managed_mode_requires_state: true,
    };

    assert_eq!(value.state_storage, AddonStateStorageValue::AppData);
    assert!(value.scan_only_without_managed_state);
    assert!(value.managed_mode_requires_state);
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

    let domain = value.clone().into_domain().expect("addon metadata");

    assert_eq!(AddonPackageMetadataValue::from_domain(domain), value);
}

#[test]
fn addon_package_metadata_value_rejects_empty_optional_text() {
    let error = AddonPackageMetadataValue {
        package_name: Some(" ".to_string()),
        ..AddonPackageMetadataValue::default()
    }
    .into_domain()
    .expect_err("empty metadata text should fail closed");

    assert!(
        error
            .to_string()
            .contains("addon package metadata package_name must not be empty")
    );
}

#[test]
fn addon_package_metadata_value_rejects_empty_supported_flavor() {
    let error = AddonPackageMetadataValue {
        supported_flavors: vec!["retail".to_string(), " ".to_string()],
        ..AddonPackageMetadataValue::default()
    }
    .into_domain()
    .expect_err("empty supported flavor should fail closed");

    assert!(
        error
            .to_string()
            .contains("addon package metadata supported_flavors must not contain empty values")
    );
}

#[test]
fn addon_release_channel_value_roundtrips_domain_shape() {
    let value = AddonReleaseChannelValue::Alpha;

    let domain = value.into_domain();

    assert_eq!(AddonReleaseChannelValue::from_domain(domain), value);
}

#[test]
fn addon_policy_pin_value_roundtrips_domain_shape() {
    let version = AddonPolicyPinValue::Version {
        value: "1.2.3".to_string(),
    };
    let file_id = AddonPolicyPinValue::FileId { value: 42 };

    assert_eq!(
        AddonPolicyPinValue::from_domain(version.clone().into_domain()),
        version
    );
    assert_eq!(
        AddonPolicyPinValue::from_domain(file_id.clone().into_domain()),
        file_id
    );
    assert_eq!(
        AddonPolicyPinValue::from_domain(AddonPolicyPin::Version {
            value: "4.5.6".to_string()
        }),
        AddonPolicyPinValue::Version {
            value: "4.5.6".to_string()
        }
    );
    assert_eq!(
        AddonReleaseChannelValue::from_domain(AddonReleaseChannel::Beta),
        AddonReleaseChannelValue::Beta
    );
}

#[test]
fn addon_dependency_resolution_values_project_domain_shape() {
    assert_eq!(
        AddonDependencyResolutionStrategyValue::from_domain(
            AddonDependencyResolutionStrategy::MissingRequiredOnly
        ),
        AddonDependencyResolutionStrategyValue::MissingRequiredOnly
    );
    assert_eq!(
        AddonDependencyResolutionCapabilityValue::from_domain(
            AddonDependencyResolutionCapability::Unsupported
        ),
        AddonDependencyResolutionCapabilityValue::Unsupported
    );
    assert_eq!(
        AddonDependencyResolutionCapabilityValue::from_domain(
            AddonDependencyResolutionCapability::Supported {
                strategy: AddonDependencyResolutionStrategy::MissingRequiredOnly
            }
        ),
        AddonDependencyResolutionCapabilityValue::Supported {
            strategy: AddonDependencyResolutionStrategyValue::MissingRequiredOnly
        }
    );
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

fn absolute_installation_value() -> ResolvedInstallationValue {
    let product_root = std::env::current_dir()
        .expect("cwd")
        .join("World of Warcraft");
    installation_value_from_product_root(product_root)
}

fn relative_installation_value() -> ResolvedInstallationValue {
    installation_value_from_product_root(PathBuf::from("World of Warcraft"))
}

fn installation_value_from_product_root(product_root: PathBuf) -> ResolvedInstallationValue {
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");

    ResolvedInstallationValue {
        platform: HostPlatformValue::Windows,
        flavor: WowFlavorValue::Retail,
        product_root,
        flavor_root: flavor_root.clone(),
        interface_dir: interface_dir.clone(),
        addon_dir: interface_dir.join("AddOns"),
        wtf_dir: flavor_root.join("WTF"),
        fonts_dir: flavor_root.join("Fonts"),
    }
}
