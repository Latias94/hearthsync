use crate::core::app::{
    AppRuntimeDiagnosticsValue, InstallationHealthResult, InstallationInspectionResult,
    InstallationScanResult,
};

use crate::cli::system::{ManifestExampleResult, ManifestValidationResult};

pub(in crate::cli) fn render_installation_scan(item: &InstallationScanResult) -> String {
    if item.installations.is_empty() {
        "No World of Warcraft installations detected.".to_string()
    } else {
        let mut lines = vec![format!(
            "Detected {} installation(s):",
            item.installation_count
        )];
        for installation in &item.installations {
            lines.push(format!(
                "- {} => {}",
                installation.flavor.as_str(),
                installation.flavor_root.display()
            ));
        }
        lines.join("\n")
    }
}

pub(in crate::cli) fn render_installation_inspection(
    item: &InstallationInspectionResult,
) -> String {
    format!(
        "Flavor: {}\nProduct root: {}\nFlavor root: {}\nAddOns: {}\nWTF: {}\nFonts: {}\nHealth: {}",
        item.installation.flavor.as_str(),
        item.product_root.display(),
        item.installation.flavor_root.display(),
        item.installation.addon_dir.display(),
        item.installation.wtf_dir.display(),
        item.installation.fonts_dir.display(),
        item.health.status_label
    )
}

pub(in crate::cli) fn render_installation_health_report(
    health: &InstallationHealthResult,
) -> String {
    let mut lines = vec![format!("Status: {}", health.status_label)];

    if health.missing_paths.is_empty() {
        lines.push("Missing required paths: none".to_string());
    } else {
        lines.push("Missing required paths:".to_string());
        for path in &health.missing_paths {
            lines.push(format!("- {}", path.display()));
        }
    }

    if health.warnings.is_empty() {
        lines.push("Warnings: none".to_string());
    } else {
        lines.push("Warnings:".to_string());
        for warning in &health.warnings {
            lines.push(format!("- {warning}"));
        }
    }

    lines.join("\n")
}

pub(in crate::cli) fn render_manifest_example(item: &ManifestExampleResult) -> String {
    item.content.trim_end().to_string()
}

pub(in crate::cli) fn render_manifest_validation(item: &ManifestValidationResult) -> String {
    format!("Manifest is valid: {}", item.path.display())
}

pub(in crate::cli) fn render_runtime_diagnostics(item: &AppRuntimeDiagnosticsValue) -> String {
    let mut lines = vec![format!(
        "Host platform: {}",
        format_platform(item.host_platform)
    )];

    if let Some(installation) = &item.selected_installation {
        lines.push(format!(
            "Runtime installation context: {} => {}",
            installation.flavor.as_str(),
            installation.flavor_root.display()
        ));
    } else {
        lines.push(
            "Runtime installation context: none (pass --install to inspect exact managed-state paths)"
                .to_string(),
        );
    }

    lines.push(format!(
        "Addon state storage: {}",
        format_addon_state_storage(item.capabilities.addon_management.state_storage)
    ));

    match &item.addon_state_paths {
        Some(paths) => {
            lines.push(format!(
                "Managed addon state root: {}",
                paths.root_dir.display()
            ));
            lines.push(format!(
                "Managed addon registry path: {}",
                paths.registry_path.display()
            ));
            lines.push(format!(
                "Managed addon lock path: {}",
                paths.lock_path.display()
            ));
            lines.push(format!(
                "Managed addon policy path: {}",
                paths.policy_path.display()
            ));
            lines.push(format!(
                "Managed adopted archive dir: {}",
                paths.adopted_dir.display()
            ));
        }
        None => lines.push(
            "Managed addon state paths: resolve with --install to inspect exact locations"
                .to_string(),
        ),
    }

    lines.push(format!(
        "Scan-only without managed state: {}",
        item.capabilities
            .addon_management
            .scan_only_without_managed_state
    ));
    lines.push(format!(
        "Managed mode requires state: {}",
        item.capabilities
            .addon_management
            .managed_mode_requires_state
    ));

    match &item.install_scan_roots {
        Some(roots) if !roots.is_empty() => {
            lines.push("Install scan roots:".to_string());
            for root in roots {
                lines.push(format!("- {}", root.display()));
            }
        }
        _ => lines.push("Install scan roots: host defaults".to_string()),
    }

    lines.push(format!(
        "Default backup dir: {}",
        item.default_backup_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    lines.push(format!(
        "Default bundle output dir: {}",
        item.default_bundle_output_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    lines.push(format!(
        "Addon provider mode: {}",
        format_addon_provider_mode(&item.capabilities)
    ));
    lines.push(format!(
        "External helper policy: {}",
        format_external_helper_policy(item.capabilities.external_helper.policy)
    ));
    lines.push(format!(
        "External helper availability: {}",
        format_external_helper_availability(item.capabilities.external_helper.availability)
    ));
    lines.push(format!(
        "Active helper strategy: {}",
        format_helper_strategy(item.capabilities.external_helper.active_strategy)
    ));

    lines.join("\n")
}

fn format_platform(value: crate::core::app::HostPlatformValue) -> &'static str {
    match value {
        crate::core::app::HostPlatformValue::Windows => "windows",
        crate::core::app::HostPlatformValue::MacOs => "macos",
        crate::core::app::HostPlatformValue::Linux => "linux",
        crate::core::app::HostPlatformValue::Unknown => "unknown",
    }
}

fn format_addon_state_storage(value: crate::core::app::AddonStateStorageValue) -> &'static str {
    match value {
        crate::core::app::AddonStateStorageValue::AppData => "app_data",
        crate::core::app::AddonStateStorageValue::Sidecar => "sidecar",
    }
}

fn format_addon_provider_mode(value: &crate::core::app::AppRuntimeCapabilitiesValue) -> String {
    match &value.addon_provider {
        crate::core::app::AddonProviderModeValue::ConfiguredDefault { options } => format!(
            "configured_default (cache: {}, max_attempts: {}, no_validator_http_cache: {})",
            options
                .download_cache_dir
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string()),
            options.retry_policy.max_attempts,
            format_http_no_validator_cache_policy(&options.http_no_validator_cache_policy)
        ),
        crate::core::app::AddonProviderModeValue::InternalCustom => "internal_custom".to_string(),
    }
}

fn format_http_no_validator_cache_policy(
    value: &crate::core::app::HttpNoValidatorCachePolicyValue,
) -> String {
    match value {
        crate::core::app::HttpNoValidatorCachePolicyValue::AlwaysRefresh => {
            "always_refresh".to_string()
        }
        crate::core::app::HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs } => {
            format!("reuse_within_window({max_age_secs}s)")
        }
    }
}

fn format_external_helper_policy(
    value: crate::core::app::ExternalHelperPolicyValue,
) -> &'static str {
    match value {
        crate::core::app::ExternalHelperPolicyValue::NativeOnly => "native_only",
        crate::core::app::ExternalHelperPolicyValue::PreferExternal => "prefer_external",
    }
}

fn format_external_helper_availability(
    value: crate::core::app::ExternalHelperAvailabilityValue,
) -> &'static str {
    match value {
        crate::core::app::ExternalHelperAvailabilityValue::NotRequested => "not_requested",
        crate::core::app::ExternalHelperAvailabilityValue::Unavailable => "unavailable",
    }
}

fn format_helper_strategy(value: crate::core::app::HelperStrategyValue) -> &'static str {
    match value {
        crate::core::app::HelperStrategyValue::NativeRust => "native_rust",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::sample_installation;
    use super::{
        render_installation_health_report, render_installation_inspection,
        render_installation_scan, render_manifest_example, render_manifest_validation,
        render_runtime_diagnostics,
    };
    use crate::cli::system::{ManifestExampleResult, ManifestValidationResult};
    use crate::core::app::{
        AddonManagementCapabilitiesValue, AddonProviderModeValue, AddonProviderOptionsValue,
        AddonProviderRetryPolicyValue, AddonStatePathsValue, AddonStateStorageValue,
        AppRuntimeCapabilitiesValue, AppRuntimeDiagnosticsValue, ExternalHelperAvailabilityValue,
        ExternalHelperCapabilitiesValue, ExternalHelperPolicyValue, HealthStatusValue,
        HelperStrategyValue, HostPlatformValue, HttpNoValidatorCachePolicyValue,
        InstallationHealthResult, InstallationInspectionResult, InstallationScanResult,
        WowFlavorValue,
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
            default_backup_dir: Some(PathBuf::from("E:\\Backups")),
            default_bundle_output_dir: None,
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
        assert!(rendered.contains("Default backup dir: E:\\Backups"));
        assert!(rendered.contains("Default bundle output dir: none"));
        assert!(rendered.contains("Addon provider mode: configured_default"));
        assert!(rendered.contains("cache: E:\\Cache"));
        assert!(rendered.contains("max_attempts: 3"));
        assert!(rendered.contains("no_validator_http_cache: reuse_within_window(600s)"));
        assert!(rendered.contains("External helper policy: prefer_external"));
        assert!(rendered.contains("External helper availability: unavailable"));
        assert!(rendered.contains("Active helper strategy: native_rust"));
    }

    #[test]
    fn render_runtime_diagnostics_without_installation_context_emits_operator_hint() {
        let rendered = render_runtime_diagnostics(&AppRuntimeDiagnosticsValue {
            host_platform: HostPlatformValue::Windows,
            install_scan_roots: None,
            default_backup_dir: None,
            default_bundle_output_dir: None,
            selected_installation: None,
            addon_state_paths: None,
            capabilities: AppRuntimeCapabilitiesValue {
                addon_provider: AddonProviderModeValue::ConfiguredDefault {
                    options: AddonProviderOptionsValue::default(),
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
            },
        });

        assert!(rendered.contains("Runtime installation context: none"));
        assert!(rendered.contains(
            "Managed addon state paths: resolve with --install to inspect exact locations"
        ));
    }
}
