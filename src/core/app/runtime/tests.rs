use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use crate::core::addon::{AddonStateStorageKind, DefaultAddonProvider};
use crate::core::app::{
    AddonManagementCapabilitiesValue, AddonProviderModeValue, AddonProviderOptionsValue,
    AddonProviderRetryPolicyValue, AddonStateStorageValue, AppRuntime, AppRuntimeCapabilitiesValue,
    ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue, ExternalHelperPolicyValue,
    HelperStrategyValue, HostPlatformValue, HttpNoValidatorCachePolicyValue,
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
fn runtime_defaults_addon_state_storage_to_appdata() {
    let temp = tempdir().expect("temp dir");
    let installation = sample_installation(temp.path());
    let runtime = AppRuntime::new();
    let state_paths = runtime
        .addon_state_paths(&installation)
        .expect("addon state paths");

    assert_eq!(
        runtime.addon_state_storage_kind(),
        AddonStateStorageKind::AppData
    );
    assert!(
        state_paths.root_dir.ends_with("retail\\addons")
            || state_paths.root_dir.ends_with("retail/addons")
    );
    assert!(!state_paths.root_dir.starts_with(&installation.addon_dir));
}

#[test]
fn runtime_can_override_addon_state_storage_to_sidecar() {
    let temp = tempdir().expect("temp dir");
    let installation = sample_installation(temp.path());
    let runtime = AppRuntime::new().with_addon_state_storage_kind(AddonStateStorageKind::Sidecar);
    let state_paths = runtime
        .addon_state_paths(&installation)
        .expect("addon state paths");

    assert_eq!(
        runtime.addon_state_storage_kind(),
        AddonStateStorageKind::Sidecar
    );
    assert_eq!(
        state_paths.root_dir,
        installation.addon_dir.join(".hearthsync")
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
    let temp = tempdir().expect("temp dir");
    let runtime = AppRuntime::builder()
        .with_relative_path_base(Some(temp.path().to_path_buf()))
        .with_addon_provider_options(AddonProviderOptionsValue {
            download_cache_dir: Some(PathBuf::from("cache")),
            retry_policy: AddonProviderRetryPolicyValue { max_attempts: 3 },
            http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue::ReuseWithinWindow {
                max_age_secs: 600,
            },
        })
        .with_external_helper_policy(ExternalHelperPolicyValue::NativeOnly)
        .build()
        .expect("runtime");

    assert_eq!(
        runtime.capabilities(),
        AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::ConfiguredDefault {
                options: AddonProviderOptionsValue {
                    download_cache_dir: Some(temp.path().join("cache")),
                    retry_policy: AddonProviderRetryPolicyValue { max_attempts: 3 },
                    http_no_validator_cache_policy:
                        HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 600 },
                },
            },
            addon_management: AddonManagementCapabilitiesValue {
                state_storage: AddonStateStorageValue::AppData,
                scan_only_without_managed_state: true,
                managed_mode_requires_state: true,
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
fn runtime_builder_rejects_relative_provider_cache_without_runtime_base() {
    let error = AppRuntime::with_addon_provider_options(AddonProviderOptionsValue {
        download_cache_dir: Some(PathBuf::from("cache")),
        ..AddonProviderOptionsValue::default()
    })
    .expect_err("relative cache path should fail closed");

    assert!(
        error
            .to_string()
            .contains("addon cache directory relative path requires")
    );
}

#[test]
fn runtime_builder_resolves_relative_runtime_paths_before_diagnostics() {
    let temp = tempdir().expect("temp dir");
    let runtime = AppRuntime::builder()
        .with_relative_path_base(Some(temp.path().to_path_buf()))
        .with_install_scan_roots(Some(vec![PathBuf::from("World of Warcraft")]))
        .with_default_backup_dir(Some(PathBuf::from("backups")))
        .with_default_bundle_output_dir(Some(PathBuf::from("bundles")))
        .build()
        .expect("runtime");

    let diagnostics = runtime.diagnostics();

    assert_eq!(
        diagnostics.install_scan_roots,
        Some(vec![temp.path().join("World of Warcraft")])
    );
    assert_eq!(
        diagnostics.default_backup_dir,
        Some(temp.path().join("backups"))
    );
    assert_eq!(
        diagnostics.default_bundle_output_dir,
        Some(temp.path().join("bundles"))
    );
}

#[test]
fn runtime_capabilities_report_internal_custom_provider_when_injected() {
    let runtime = AppRuntime::with_addon_provider(DefaultAddonProvider::default());

    assert_eq!(
        runtime.capabilities(),
        AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::InternalCustom,
            addon_management: AddonManagementCapabilitiesValue {
                state_storage: AddonStateStorageValue::AppData,
                scan_only_without_managed_state: true,
                managed_mode_requires_state: true,
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
fn runtime_defaults_external_helper_to_native_rust_without_requesting_external_support() {
    assert_eq!(
        AppRuntime::new().capabilities(),
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
fn runtime_capabilities_project_sidecar_addon_management_backend() {
    let runtime = AppRuntime::new().with_addon_state_storage_kind(AddonStateStorageKind::Sidecar);

    assert_eq!(
        runtime.addon_management_capabilities(),
        AddonManagementCapabilitiesValue {
            state_storage: AddonStateStorageValue::Sidecar,
            scan_only_without_managed_state: true,
            managed_mode_requires_state: true,
        }
    );
}

#[test]
fn runtime_diagnostics_for_installation_projects_exact_addon_state_paths() {
    let temp = tempdir().expect("temp dir");
    let installation = sample_installation(temp.path());
    let runtime = AppRuntime::new();

    let diagnostics = runtime
        .diagnostics_for_installation(crate::core::app::ResolvedInstallationValue::from_domain(
            installation.clone(),
        ))
        .expect("runtime diagnostics");

    assert_eq!(
        diagnostics.selected_installation,
        Some(crate::core::app::ResolvedInstallationValue::from_domain(
            installation.clone()
        ))
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
fn runtime_defaults_provider_options_to_default_configured_mode() {
    assert_eq!(
        AppRuntime::new().capabilities().addon_provider,
        AddonProviderModeValue::ConfiguredDefault {
            options: AddonProviderOptionsValue {
                download_cache_dir: None,
                retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
                http_no_validator_cache_policy:
                    HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 900 },
            },
        }
    );
}

fn sample_installation(root: &std::path::Path) -> crate::core::install::DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");

    crate::core::install::DetectedFlavorInstallation {
        platform: crate::core::install::HostPlatform::Windows,
        product_root,
        flavor_root: flavor_root.clone(),
        flavor: crate::core::install::WowFlavor::Retail,
        interface_dir: flavor_root.join("Interface"),
        addon_dir: flavor_root.join("Interface").join("AddOns"),
        wtf_dir: flavor_root.join("WTF"),
        fonts_dir: flavor_root.join("Fonts"),
    }
}
