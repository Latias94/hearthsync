use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::core::app::{
    AddonManagementCapabilitiesValue, AddonProviderModeValue, AddonProviderOptionsValue,
    AddonProviderRetryPolicyValue, AddonStateStorageValue, AppRuntime, AppRuntimeCapabilitiesValue,
    ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue, ExternalHelperPolicyValue,
    HealthStatusValue, HelperStrategyValue, HostPlatformValue, HttpNoValidatorCachePolicyValue,
    InspectInstallationRequest, ResolveInstallationRequest, SetRuntimeSettingsAppRequest,
    StableAppServices, WowFlavorValue, runtime_settings_path_guard,
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
    assert_eq!(
        services.addon_policies().runtime().host_platform(),
        HostPlatformValue::MacOs
    );
    assert!(std::ptr::eq(
        services.installations(),
        services.installations()
    ));
    assert!(std::ptr::eq(services.addons(), services.addons()));
    assert!(std::ptr::eq(
        services.addon_policies(),
        services.addon_policies()
    ));
    assert!(std::ptr::eq(services.backups(), services.backups()));
    assert!(std::ptr::eq(services.bundles(), services.bundles()));
    assert!(std::ptr::eq(
        services.external_packages(),
        services.external_packages()
    ));
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
fn stable_app_services_expose_runtime_diagnostics_as_app_owned_value() {
    let runtime = AppRuntime::new()
        .with_host_platform(HostPlatformValue::MacOs)
        .with_install_scan_roots(Some(vec![Path::new("/wow").to_path_buf()]))
        .with_default_backup_dir(Some(Path::new("/backups").to_path_buf()))
        .with_default_bundle_output_dir(Some(Path::new("/bundles").to_path_buf()));
    let services = StableAppServices::with_runtime(runtime);

    let diagnostics = services.runtime_diagnostics();

    assert_eq!(diagnostics.host_platform, HostPlatformValue::MacOs);
    assert_eq!(
        diagnostics.install_scan_roots,
        Some(vec![Path::new("/wow").to_path_buf()])
    );
    assert_eq!(
        diagnostics.default_backup_dir,
        Some(Path::new("/backups").to_path_buf())
    );
    assert_eq!(
        diagnostics.default_bundle_output_dir,
        Some(Path::new("/bundles").to_path_buf())
    );
    assert_eq!(diagnostics.selected_installation, None);
    assert_eq!(diagnostics.addon_state_paths, None);
    assert_eq!(
        diagnostics.capabilities.addon_management.state_storage,
        crate::core::app::AddonStateStorageValue::AppData
    );
}

#[test]
fn stable_app_services_runtime_diagnostics_can_project_exact_addon_state_paths() {
    let temp = tempdir().expect("temp dir");
    let product_root = temp.path().join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
    fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");

    let services = StableAppServices::with_runtime(AppRuntime::new());
    let installation = services
        .resolve_installation(ResolveInstallationRequest {
            path: product_root,
            flavor: Some(WowFlavorValue::Retail),
        })
        .expect("resolve installation");

    let diagnostics = services
        .runtime_diagnostics_for_installation(installation.clone())
        .expect("runtime diagnostics");

    assert_eq!(
        diagnostics.selected_installation,
        Some(installation.clone())
    );

    let addon_state_paths = diagnostics.addon_state_paths.expect("addon state paths");
    let root = addon_state_paths
        .root_dir
        .to_string_lossy()
        .replace('\\', "/");
    assert!(root.contains("/wow/"));
    assert!(root.contains("/retail/addons"));
    assert_eq!(
        addon_state_paths.registry_path,
        addon_state_paths.root_dir.join("addons.toml")
    );
    assert_eq!(
        addon_state_paths.lock_path,
        addon_state_paths.root_dir.join("lock.toml")
    );
    assert_eq!(
        addon_state_paths.policy_path,
        addon_state_paths.root_dir.join("addon-policy.toml")
    );
    assert_eq!(
        addon_state_paths.adopted_dir,
        addon_state_paths.root_dir.join("adopted")
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

#[test]
fn stable_app_services_expose_runtime_settings_entrypoints() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let services = StableAppServices::with_runtime(AppRuntime::new());

    let initial = services
        .inspect_runtime_settings()
        .expect("inspect initial settings");
    assert!(!initial.settings_file_exists);
    assert_eq!(initial.settings_path, settings_path);
    assert_eq!(
        initial.settings,
        crate::core::app::RuntimeSettingsValue::default()
    );

    let mutation = services
        .set_runtime_settings(SetRuntimeSettingsAppRequest {
            addon_state_storage: Some(AddonStateStorageValue::Sidecar),
            clear_addon_state_storage: false,
            addon_cache_dir: Some(Path::new("/cache").to_path_buf()),
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: Some(
                HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 300 },
            ),
            clear_http_no_validator_cache_policy: false,
        })
        .expect("set settings");
    assert!(mutation.settings_file_exists);
    assert_eq!(mutation.settings_path, settings_path);
    assert_eq!(
        mutation.settings.addon_state_storage,
        Some(AddonStateStorageValue::Sidecar)
    );

    let inspection = services
        .inspect_runtime_settings()
        .expect("inspect persisted settings");
    assert!(inspection.settings_file_exists);
    assert_eq!(inspection.settings, mutation.settings);

    let reset = services.reset_runtime_settings().expect("reset settings");
    assert!(reset.file_removed);
    assert!(!reset.settings_file_exists);
    assert_eq!(reset.settings_path, settings_path);
    assert_eq!(
        reset.settings,
        crate::core::app::RuntimeSettingsValue::default()
    );
}
