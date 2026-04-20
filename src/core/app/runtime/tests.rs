use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use crate::core::addon::DefaultAddonProvider;
use crate::core::app::{
    AddonProviderModeValue, AddonProviderOptionsValue, AddonProviderRetryPolicyValue, AppRuntime,
    AppRuntimeCapabilitiesValue, ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue,
    ExternalHelperPolicyValue, HelperStrategyValue, HostPlatformValue,
};

#[test]
fn runtime_default_helpers_preserve_explicit_paths_and_fill_missing_ones() {
    let backup_dir = PathBuf::from("backups");
    let bundle_dir = PathBuf::from("bundles");
    let explicit_backup = PathBuf::from("custom-backups");
    let explicit_bundle = PathBuf::from("custom-bundles");
    let runtime = AppRuntime::new()
        .with_default_backup_dir(Some(backup_dir.clone()))
        .with_default_bundle_output_dir(Some(bundle_dir.clone()));

    assert_eq!(
        runtime.backup_output_or_default(None),
        Some(backup_dir.clone())
    );
    assert_eq!(
        runtime.backup_output_or_default(Some(explicit_backup.clone())),
        Some(explicit_backup)
    );
    assert_eq!(runtime.backup_dir_or_default(None), Some(backup_dir));
    assert_eq!(
        runtime.bundle_output_or_default(None),
        Some(bundle_dir.clone())
    );
    assert_eq!(
        runtime.bundle_output_or_default(Some(explicit_bundle.clone())),
        Some(explicit_bundle)
    );
}

#[test]
fn runtime_source_platform_or_host_uses_explicit_platform_before_host_default() {
    let runtime = AppRuntime::new().with_host_platform(HostPlatformValue::MacOs);

    assert_eq!(
        runtime.source_platform_or_host(None),
        HostPlatformValue::MacOs
    );
    assert_eq!(
        runtime.source_platform_or_host(Some(HostPlatformValue::Windows)),
        HostPlatformValue::Windows
    );
}

#[test]
fn runtime_scan_installations_uses_configured_roots_and_host_platform() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");

    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_install_scan_roots(Some(vec![product_root.clone()]));
    let installations = runtime.scan_installations().expect("scan installations");

    assert_eq!(installations.len(), 1);
    assert_eq!(
        installations[0].platform,
        crate::core::install::HostPlatform::MacOs
    );
    assert_eq!(installations[0].product_root, product_root);
}

#[test]
fn runtime_capabilities_report_configured_default_provider_and_external_helper_state() {
    let runtime = AppRuntime::with_addon_provider_options(AddonProviderOptionsValue {
        download_cache_dir: Some(PathBuf::from("cache")),
        retry_policy: AddonProviderRetryPolicyValue { max_attempts: 3 },
    })
    .with_external_helper_policy(ExternalHelperPolicyValue::NativeOnly);

    assert_eq!(
        runtime.capabilities(),
        AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::ConfiguredDefault {
                options: AddonProviderOptionsValue {
                    download_cache_dir: Some(PathBuf::from("cache")),
                    retry_policy: AddonProviderRetryPolicyValue { max_attempts: 3 },
                },
            },
            external_helper: ExternalHelperCapabilitiesValue {
                policy: ExternalHelperPolicyValue::NativeOnly,
                availability: ExternalHelperAvailabilityValue::NotRequested,
                active_strategy: HelperStrategyValue::NativeRust,
            },
        }
    );
}

#[test]
fn runtime_capabilities_report_internal_custom_provider_when_injected() {
    let runtime = AppRuntime::with_addon_provider(DefaultAddonProvider::default());

    assert_eq!(
        runtime.capabilities(),
        AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::InternalCustom,
            external_helper: ExternalHelperCapabilitiesValue {
                policy: ExternalHelperPolicyValue::NativeOnly,
                availability: ExternalHelperAvailabilityValue::NotRequested,
                active_strategy: HelperStrategyValue::NativeRust,
            },
        }
    );
}

#[test]
fn runtime_defaults_external_helper_to_native_rust_without_requesting_external_support() {
    assert_eq!(
        AppRuntime::new().capabilities(),
        AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::ConfiguredDefault {
                options: AddonProviderOptionsValue {
                    download_cache_dir: None,
                    retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
                },
            },
            external_helper: ExternalHelperCapabilitiesValue {
                policy: ExternalHelperPolicyValue::NativeOnly,
                availability: ExternalHelperAvailabilityValue::NotRequested,
                active_strategy: HelperStrategyValue::NativeRust,
            },
        }
    );
    assert_eq!(
        AppRuntime::new().helper_strategy(),
        HelperStrategyValue::NativeRust
    );
}

#[test]
fn runtime_capabilities_report_unavailable_external_helper_when_preferred() {
    let runtime =
        AppRuntime::new().with_external_helper_policy(ExternalHelperPolicyValue::PreferExternal);

    assert_eq!(
        runtime.external_helper_policy(),
        ExternalHelperPolicyValue::PreferExternal
    );
    assert_eq!(
        runtime.external_helper_capabilities(),
        ExternalHelperCapabilitiesValue {
            policy: ExternalHelperPolicyValue::PreferExternal,
            availability: ExternalHelperAvailabilityValue::Unavailable,
            active_strategy: HelperStrategyValue::NativeRust,
        }
    );
    assert_eq!(runtime.helper_strategy(), HelperStrategyValue::NativeRust);
}

#[test]
fn runtime_defaults_provider_options_to_default_configured_mode() {
    assert_eq!(
        AppRuntime::new().capabilities().addon_provider,
        AddonProviderModeValue::ConfiguredDefault {
            options: AddonProviderOptionsValue {
                download_cache_dir: None,
                retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
            },
        }
    );
}
