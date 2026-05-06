use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::addon::policy::{AddonPolicyPin, AddonReleaseChannel};
use crate::core::addon::{
    AddonDependencyResolutionCapability, AddonDependencyResolutionStrategy, AddonStatePaths,
    AddonStateStorageKind,
};
use crate::core::app::{
    AddonCacheRepairRemotePolicyValue, AddonDependencyResolutionCapabilityValue,
    AddonDependencyResolutionStrategyValue, AddonManagementCapabilitiesValue,
    AddonPackageMetadataValue, AddonPolicyPinValue, AddonProviderOptionsValue,
    AddonProviderRetryPolicyValue, AddonReleaseChannelValue, AddonSourceFamilyValue,
    AddonStatePathsValue, AddonStateStorageValue, HttpNoValidatorCachePolicyValue,
};

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
        cache_repair_remote_policy: AddonCacheRepairRemotePolicyValue::RequireRemote,
        search_cache_ttl_secs: 60,
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
fn addon_cache_repair_remote_policy_value_roundtrips_domain_shape() {
    let value = AddonCacheRepairRemotePolicyValue::RequireRemote;

    let domain = value.into_domain();

    assert_eq!(
        AddonCacheRepairRemotePolicyValue::from_domain(domain),
        value
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
fn addon_source_family_value_accepts_future_provider_family_ids() {
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct SourceFamilyFixture {
        source_family: AddonSourceFamilyValue,
    }

    let fixture: SourceFamilyFixture =
        toml::from_str(r#"source_family = "wago_addon""#).expect("source family fixture");

    assert_eq!(fixture.source_family.as_str(), "wago_addon");
    assert_eq!(
        toml::to_string(&fixture).expect("serialize source family"),
        "source_family = \"wago_addon\"\n"
    );
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
