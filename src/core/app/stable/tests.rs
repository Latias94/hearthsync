use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::core::app::{
    AddonProviderModeValue, AddonProviderOptionsValue, AddonProviderRetryPolicyValue,
    AppRuntime, AppRuntimeCapabilitiesValue, ExternalHelperAvailabilityValue,
    ExternalHelperCapabilitiesValue, ExternalHelperPolicyValue, HealthStatusValue,
    HelperStrategyValue, HostPlatformValue, InspectInstallationRequest,
    ResolveInstallationRequest, StableAppServices, WowFlavorValue,
};

#[test]
fn stable_app_services_share_runtime_with_first_wave_gui_services() {
    let temp = tempdir().expect("temp dir");
    let scan_root = temp.path().join("scan-root");
    let backup_dir = temp.path().join("backups");
    let bundle_dir = temp.path().join("bundles");
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_install_scan_roots(Some(vec![scan_root.clone()]))
        .with_default_backup_dir(Some(backup_dir.clone()))
        .with_default_bundle_output_dir(Some(bundle_dir.clone()));

    let services = StableAppServices::with_runtime(runtime);

    assert_eq!(
        services.installations().runtime().install_scan_roots(),
        Some([scan_root].as_slice())
    );
    assert_eq!(
        services.installations().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
    assert_eq!(
        services.backups().runtime().default_backup_dir(),
        Some(backup_dir.as_path())
    );
    assert_eq!(
        services.bundles().runtime().default_bundle_output_dir(),
        Some(bundle_dir.as_path())
    );
    assert_eq!(
        services
            .external_packages()
            .runtime()
            .default_bundle_output_dir(),
        Some(bundle_dir.as_path())
    );
    assert_eq!(
        services.addons().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
}

#[test]
fn stable_app_services_expose_runtime_capabilities_as_app_owned_value() {
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_external_helper_policy(ExternalHelperPolicyValue::PreferExternal);
    let services = StableAppServices::with_runtime(runtime);

    assert_eq!(
        services.capabilities(),
        AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::ConfiguredDefault {
                options: AddonProviderOptionsValue {
                    download_cache_dir: None,
                    retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
                },
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
fn stable_app_services_direct_installation_entrypoints_use_shared_runtime() {
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

    let services = StableAppServices::with_runtime(
        AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_install_scan_roots(Some(vec![product_root.clone()])),
    );

    let scanned = services.scan_installations().expect("scan installations");
    let inspected = services
        .inspect_installation(InspectInstallationRequest {
            path: product_root.clone(),
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("inspect installation");
    let resolved = services
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
