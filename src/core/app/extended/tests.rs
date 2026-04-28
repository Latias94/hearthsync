use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::core::app::{
    AddonManagementCapabilitiesValue, AddonProviderModeValue, AddonProviderOptionsValue,
    AddonProviderRetryPolicyValue, AddonStateStorageValue, AppRuntime, AppRuntimeCapabilitiesValue,
    ExtendedAppServices, ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue,
    ExternalHelperPolicyValue, HealthStatusValue, HelperStrategyValue, HostPlatformValue,
    HttpNoValidatorCachePolicyValue, InspectInstallationRequest, ResolveInstallationRequest,
    WowFlavorValue,
};

#[test]
fn extended_app_services_builds_services_with_shared_runtime() {
    let temp = tempdir().expect("temp dir");
    let scan_root = temp.path().join("scan-root");
    let backup_dir = temp.path().join("backups");
    let bundle_dir = temp.path().join("bundles");
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_install_scan_roots(Some(vec![scan_root.clone()]))
        .with_default_backup_dir(Some(backup_dir.clone()))
        .with_default_bundle_output_dir(Some(bundle_dir.clone()));

    let app = ExtendedAppServices::with_runtime(runtime);
    let stable = app.stable();

    assert_eq!(
        stable.installations().runtime().install_scan_roots(),
        Some([scan_root].as_slice())
    );
    assert_eq!(
        stable.installations().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
    assert_eq!(
        stable.backups().runtime().default_backup_dir(),
        Some(backup_dir.as_path())
    );
    assert_eq!(
        stable.bundles().runtime().default_bundle_output_dir(),
        Some(bundle_dir.as_path())
    );
    assert_eq!(
        stable
            .external_packages()
            .runtime()
            .default_bundle_output_dir(),
        Some(bundle_dir.as_path())
    );
    assert_eq!(
        stable.addons().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
    assert_eq!(
        app.addon_indexes().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
    assert_eq!(
        app.addon_locks().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
    assert!(std::ptr::eq(app.addon_indexes(), app.addon_indexes()));
    assert!(std::ptr::eq(app.addon_locks(), app.addon_locks()));
}

#[test]
fn extended_app_services_exposes_first_wave_stable_services() {
    let temp = tempdir().expect("temp dir");
    let backup_dir = temp.path().join("backups");
    let bundle_dir = temp.path().join("bundles");
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_default_backup_dir(Some(backup_dir.clone()))
        .with_default_bundle_output_dir(Some(bundle_dir.clone()));

    let app = ExtendedAppServices::with_runtime(runtime);
    let stable = app.stable();

    assert_eq!(stable.runtime().host_platform(), HostPlatformValue::MacOs);
    assert_eq!(
        stable.backups().runtime().default_backup_dir(),
        Some(backup_dir.as_path())
    );
    assert_eq!(
        stable.bundles().runtime().default_bundle_output_dir(),
        Some(bundle_dir.as_path())
    );
    assert_eq!(
        stable.addons().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
}

#[test]
fn extended_app_services_exposes_runtime_capabilities_as_app_owned_value() {
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_external_helper_policy(ExternalHelperPolicyValue::PreferExternal);
    let app = ExtendedAppServices::with_runtime(runtime);

    assert_eq!(
        app.stable().capabilities(),
        AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::ConfiguredDefault {
                options: AddonProviderOptionsValue {
                    download_cache_dir: None,
                    retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
                    http_no_validator_cache_policy:
                        HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 900 },
                },
            },
            addon_management: AddonManagementCapabilitiesValue {
                state_storage: AddonStateStorageValue::AppData,
                scan_only_without_managed_state: true,
                managed_mode_requires_state: true,
            },
            external_helper: ExternalHelperCapabilitiesValue {
                policy: ExternalHelperPolicyValue::PreferExternal,
                availability: ExternalHelperAvailabilityValue::Unavailable,
                active_strategy: HelperStrategyValue::NativeRust,
            },
        }
    );
}

#[test]
fn extended_app_services_stable_bridge_uses_shared_runtime_for_installation_flows() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");
    fs::write(
        flavor_root.join("WTF").join("Config.wtf"),
        "SET locale enUS",
    )
    .expect("config");

    let app = ExtendedAppServices::with_runtime(
        AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_install_scan_roots(Some(vec![product_root.clone()])),
    );

    let stable = app.stable();

    let scanned = stable.scan_installations().expect("scan installations");
    let inspected = stable
        .inspect_installation(InspectInstallationRequest {
            path: product_root.clone(),
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("inspect installation");
    let resolved = stable
        .resolve_installation(ResolveInstallationRequest {
            path: product_root,
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("resolve installation");

    assert_eq!(scanned.installation_count, 1);
    assert_eq!(scanned.installations[0].platform, HostPlatformValue::MacOs);
    assert_eq!(inspected.installation.platform, HostPlatformValue::MacOs);
    assert_eq!(inspected.health.status, HealthStatusValue::Warning);
    assert_eq!(resolved.platform, HostPlatformValue::MacOs);
    assert!(
        resolved
            .flavor_root
            .ends_with(Path::new("World of Warcraft").join("_retail_"))
    );
}
