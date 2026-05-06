use std::path::PathBuf;

use super::{
    render_installation_health_report, render_installation_inspection, render_installation_scan,
    render_manifest_example, render_manifest_validation, render_runtime_diagnostics,
};
use crate::cli::output::test_support::sample_installation;
use crate::cli::system::{ManifestExampleResult, ManifestValidationResult};
use crate::core::app::{
    AddonCacheRepairRemotePolicyValue, AddonManagementCapabilitiesValue, AddonProviderModeValue,
    AddonProviderOptionsValue, AddonProviderRetryPolicyValue, AddonStatePathsValue,
    AddonStateStorageValue, AppRuntime, AppRuntimeCapabilitiesValue, AppRuntimeDiagnosticsValue,
    ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue, ExternalHelperPolicyValue,
    HealthStatusValue, HelperStrategyValue, HostPlatformValue, HttpNoValidatorCachePolicyValue,
    InstallationHealthResult, InstallationInspectionResult, InstallationScanResult,
    NetworkProxyDiagnosticsValue, ProviderCredentialDiagnosticsValue, WowFlavorValue,
};

#[test]
fn render_installation_scan_lists_detected_installations() {
    let rendered = render_installation_scan(&InstallationScanResult {
        installation_count: 1,
        installations: vec![sample_installation()],
    });

    assert!(rendered.contains("Detected 1 installation(s):"));
    assert!(rendered.contains("- retail => C:\\Games\\World of Warcraft\\_retail_"));
}

#[test]
fn render_installation_health_report_lists_missing_paths_and_warnings() {
    let rendered = render_installation_health_report(&InstallationHealthResult {
        status: HealthStatusValue::Warning,
        status_label: "warning".to_string(),
        missing_paths: vec![PathBuf::from("Fonts")],
        warnings: vec!["WTF folder is empty".to_string()],
    });

    assert!(rendered.contains("Status: warning"));
    assert!(rendered.contains("Missing required paths:"));
    assert!(rendered.contains("- Fonts"));
    assert!(rendered.contains("Warnings:"));
    assert!(rendered.contains("- WTF folder is empty"));
}

#[test]
fn render_installation_inspection_reports_selected_flavor() {
    let rendered = render_installation_inspection(&InstallationInspectionResult {
        requested_path: PathBuf::from("C:\\Games\\World of Warcraft"),
        product_root: PathBuf::from("C:\\Games\\World of Warcraft"),
        available_flavors: vec![WowFlavorValue::Retail],
        installation: sample_installation(),
        health: InstallationHealthResult {
            status: HealthStatusValue::Healthy,
            status_label: "healthy".to_string(),
            missing_paths: Vec::new(),
            warnings: Vec::new(),
        },
    });

    assert!(rendered.contains("Flavor: retail"));
    assert!(rendered.contains("Product root: C:\\Games\\World of Warcraft"));
    assert!(rendered.contains("Health: healthy"));
}

#[test]
fn render_manifest_example_returns_toml_content_without_extra_trailing_newlines() {
    let rendered = render_manifest_example(&ManifestExampleResult {
        content: "schema_version = 1\n\n".to_string(),
    });

    assert_eq!(rendered, "schema_version = 1");
}

#[test]
fn render_manifest_validation_reports_valid_path() {
    let rendered = render_manifest_validation(&ManifestValidationResult {
        status: "ok".to_string(),
        path: PathBuf::from("bundle/manifest.toml"),
    });

    assert_eq!(rendered, "Manifest is valid: bundle/manifest.toml");
}

#[test]
fn render_runtime_diagnostics_reports_runtime_settings_and_capabilities() {
    let rendered = render_runtime_diagnostics(&AppRuntimeDiagnosticsValue {
        host_platform: HostPlatformValue::Windows,
        install_scan_roots: Some(vec![PathBuf::from("E:\\Games")]),
        relative_path_base: Some(PathBuf::from("E:\\Work")),
        default_backup_dir: Some(PathBuf::from("E:\\Backups")),
        default_bundle_output_dir: None,
        network_proxy: NetworkProxyDiagnosticsValue {
            http_proxy: true,
            https_proxy: false,
            all_proxy: false,
            no_proxy: true,
        },
        provider_credentials: ProviderCredentialDiagnosticsValue {
            github_token: true,
            curseforge_api_key: true,
        },
        selected_installation: Some(sample_installation()),
        addon_state_paths: Some(AddonStatePathsValue {
            root_dir: PathBuf::from(
                "C:\\Users\\Tester\\AppData\\Local\\hearthsync\\wow\\world-of-warcraft-123456\\retail\\addons",
            ),
            registry_path: PathBuf::from(
                "C:\\Users\\Tester\\AppData\\Local\\hearthsync\\wow\\world-of-warcraft-123456\\retail\\addons\\addons.toml",
            ),
            lock_path: PathBuf::from(
                "C:\\Users\\Tester\\AppData\\Local\\hearthsync\\wow\\world-of-warcraft-123456\\retail\\addons\\lock.toml",
            ),
            policy_path: PathBuf::from(
                "C:\\Users\\Tester\\AppData\\Local\\hearthsync\\wow\\world-of-warcraft-123456\\retail\\addons\\addon-policy.toml",
            ),
            adopted_dir: PathBuf::from(
                "C:\\Users\\Tester\\AppData\\Local\\hearthsync\\wow\\world-of-warcraft-123456\\retail\\addons\\adopted",
            ),
        }),
        capabilities: AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::ConfiguredDefault {
                options: AddonProviderOptionsValue {
                    download_cache_dir: Some(PathBuf::from("E:\\Cache")),
                    retry_policy: AddonProviderRetryPolicyValue { max_attempts: 3 },
                    http_no_validator_cache_policy:
                        HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 600 },
                    cache_repair_remote_policy: AddonCacheRepairRemotePolicyValue::RequireRemote,
                    search_cache_ttl_secs: 30,
                },
            },
            addon_source_capabilities: default_addon_source_capabilities(),
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
        },
    });

    assert!(rendered.contains("Host platform: windows"));
    assert!(rendered.contains("Runtime installation context: retail =>"));
    assert!(rendered.contains("Addon state storage: app_data"));
    assert!(rendered.contains("Managed addon state root:"));
    assert!(rendered.contains("Managed addon registry path:"));
    assert!(rendered.contains("Managed addon lock path:"));
    assert!(rendered.contains("Managed addon policy path:"));
    assert!(rendered.contains("Managed adopted archive dir:"));
    assert!(rendered.contains("Scan-only without managed state: true"));
    assert!(rendered.contains("Managed mode requires state: true"));
    assert!(rendered.contains("Install scan roots:"));
    assert!(rendered.contains("- E:\\Games"));
    assert!(rendered.contains("Relative path base: E:\\Work"));
    assert!(rendered.contains("Default backup dir: E:\\Backups"));
    assert!(rendered.contains("Default bundle output dir: none"));
    assert!(rendered.contains("Network proxy signals: HTTP_PROXY, NO_PROXY"));
    assert!(rendered.contains("Provider credential signals: GitHub token, CurseForge API key"));
    assert!(rendered.contains("Addon provider mode: configured_default"));
    assert!(rendered.contains("cache: E:\\Cache"));
    assert!(rendered.contains("max_attempts: 3"));
    assert!(rendered.contains("no_validator_http_cache: reuse_within_window(600s)"));
    assert!(rendered.contains("cache_repair_remote: require_remote"));
    assert!(rendered.contains("search_cache_ttl: 30s"));
    assert!(rendered.contains("Addon source capabilities:"));
    assert!(rendered.contains("curseforge:curseforge_mod"));
    assert!(rendered.contains("github:github_release"));
    assert!(rendered.contains("External helper policy: prefer_external"));
    assert!(rendered.contains("External helper availability: unavailable"));
    assert!(rendered.contains("Active helper strategy: native_rust"));
}

#[test]
fn render_runtime_diagnostics_without_installation_context_emits_operator_hint() {
    let rendered = render_runtime_diagnostics(&AppRuntimeDiagnosticsValue {
        host_platform: HostPlatformValue::Windows,
        install_scan_roots: None,
        relative_path_base: None,
        default_backup_dir: None,
        default_bundle_output_dir: None,
        network_proxy: NetworkProxyDiagnosticsValue::default(),
        provider_credentials: ProviderCredentialDiagnosticsValue::default(),
        selected_installation: None,
        addon_state_paths: None,
        capabilities: AppRuntimeCapabilitiesValue {
            addon_provider: AddonProviderModeValue::ConfiguredDefault {
                options: AddonProviderOptionsValue::default(),
            },
            addon_source_capabilities: Vec::new(),
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
        },
    });

    assert!(rendered.contains("Runtime installation context: none"));
    assert!(
        rendered.contains(
            "Managed addon state paths: resolve with --install to inspect exact locations"
        )
    );
    assert!(rendered.contains("Network proxy signals: none"));
    assert!(rendered.contains("Provider credential signals: none"));
    assert!(rendered.contains("Addon source capabilities: none"));
}

fn default_addon_source_capabilities() -> Vec<crate::core::app::AddonProviderSourceCapabilityValue>
{
    AppRuntime::new().capabilities().addon_source_capabilities
}
