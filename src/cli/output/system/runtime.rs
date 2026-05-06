use crate::core::app::{
    AddonCacheRepairRemotePolicyValue, AddonDependencyResolutionCapabilityValue,
    AddonProviderSourceCapabilityValue, AddonStateStorageValue, AppRuntimeCapabilitiesValue,
    AppRuntimeDiagnosticsValue, ExternalHelperAvailabilityValue, ExternalHelperPolicyValue,
    HelperStrategyValue, HostPlatformValue, HttpNoValidatorCachePolicyValue,
    NetworkProxyDiagnosticsValue,
};

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
        "Relative path base: {}",
        item.relative_path_base
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string())
    ));

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
        "Network proxy signals: {}",
        format_network_proxy_diagnostics(&item.network_proxy)
    ));
    lines.push(format!(
        "Addon provider mode: {}",
        format_addon_provider_mode(&item.capabilities)
    ));
    lines.push(format!(
        "Addon source capabilities: {}",
        format_addon_source_capabilities(&item.capabilities.addon_source_capabilities)
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

fn format_addon_source_capabilities(values: &[AddonProviderSourceCapabilityValue]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }

    values
        .iter()
        .map(|value| {
            format!(
                "{}:{} parse={} materialize={} search={} dependencies={} policy={} pin={} validators={}",
                value.provider_id,
                value.source_family.as_str(),
                value.can_parse_input,
                value.can_materialize,
                value.can_search,
                format_dependency_capability(value.dependency_resolution),
                format_source_policy_capabilities(value),
                format_source_pin_capabilities(value),
                value.supports_remote_cache_validators,
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn format_dependency_capability(value: AddonDependencyResolutionCapabilityValue) -> &'static str {
    match value {
        AddonDependencyResolutionCapabilityValue::Unsupported => "unsupported",
        AddonDependencyResolutionCapabilityValue::Supported { .. } => "supported",
    }
}

fn format_source_policy_capabilities(value: &AddonProviderSourceCapabilityValue) -> String {
    let mut capabilities = Vec::new();
    if value.supports_release_channel {
        capabilities.push("release_channel");
    }
    if value.supports_prerelease {
        capabilities.push("prerelease");
    }

    if capabilities.is_empty() {
        "none".to_string()
    } else {
        capabilities.join("+")
    }
}

fn format_source_pin_capabilities(value: &AddonProviderSourceCapabilityValue) -> String {
    let mut capabilities = Vec::new();
    if value.supports_version_pin {
        capabilities.push("version");
    }
    if value.supports_file_id_pin {
        capabilities.push("file_id");
    }

    if capabilities.is_empty() {
        "none".to_string()
    } else {
        capabilities.join("+")
    }
}

fn format_platform(value: HostPlatformValue) -> &'static str {
    match value {
        HostPlatformValue::Windows => "windows",
        HostPlatformValue::MacOs => "macos",
        HostPlatformValue::Linux => "linux",
        HostPlatformValue::Unknown => "unknown",
    }
}

fn format_addon_state_storage(value: AddonStateStorageValue) -> &'static str {
    match value {
        AddonStateStorageValue::AppData => "app_data",
        AddonStateStorageValue::Sidecar => "sidecar",
    }
}

fn format_network_proxy_diagnostics(value: &NetworkProxyDiagnosticsValue) -> String {
    let mut signals = Vec::new();
    if value.http_proxy {
        signals.push("HTTP_PROXY");
    }
    if value.https_proxy {
        signals.push("HTTPS_PROXY");
    }
    if value.all_proxy {
        signals.push("ALL_PROXY");
    }
    if value.no_proxy {
        signals.push("NO_PROXY");
    }

    if signals.is_empty() {
        "none".to_string()
    } else {
        signals.join(", ")
    }
}

fn format_addon_provider_mode(value: &AppRuntimeCapabilitiesValue) -> String {
    match &value.addon_provider {
        crate::core::app::AddonProviderModeValue::ConfiguredDefault { options } => format!(
            "configured_default (cache: {}, max_attempts: {}, no_validator_http_cache: {}, cache_repair_remote: {}, search_cache_ttl: {})",
            options
                .download_cache_dir
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "none".to_string()),
            options.retry_policy.max_attempts,
            format_http_no_validator_cache_policy(&options.http_no_validator_cache_policy),
            format_addon_cache_repair_remote_policy(options.cache_repair_remote_policy),
            format_addon_search_cache_ttl_secs(options.search_cache_ttl_secs)
        ),
        crate::core::app::AddonProviderModeValue::InternalCustom => "internal_custom".to_string(),
    }
}

fn format_http_no_validator_cache_policy(value: &HttpNoValidatorCachePolicyValue) -> String {
    match value {
        HttpNoValidatorCachePolicyValue::AlwaysRefresh => "always_refresh".to_string(),
        HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs } => {
            format!("reuse_within_window({max_age_secs}s)")
        }
    }
}

fn format_addon_cache_repair_remote_policy(
    value: AddonCacheRepairRemotePolicyValue,
) -> &'static str {
    match value {
        AddonCacheRepairRemotePolicyValue::LocalOnly => "local_only",
        AddonCacheRepairRemotePolicyValue::ValidateRemote => "validate_remote",
        AddonCacheRepairRemotePolicyValue::RequireRemote => "require_remote",
    }
}

fn format_addon_search_cache_ttl_secs(value: u64) -> String {
    if value == 0 {
        "disabled".to_string()
    } else {
        format!("{value}s")
    }
}

fn format_external_helper_policy(value: ExternalHelperPolicyValue) -> &'static str {
    match value {
        ExternalHelperPolicyValue::NativeOnly => "native_only",
        ExternalHelperPolicyValue::PreferExternal => "prefer_external",
    }
}

fn format_external_helper_availability(value: ExternalHelperAvailabilityValue) -> &'static str {
    match value {
        ExternalHelperAvailabilityValue::NotRequested => "not_requested",
        ExternalHelperAvailabilityValue::Unavailable => "unavailable",
    }
}

fn format_helper_strategy(value: HelperStrategyValue) -> &'static str {
    match value {
        HelperStrategyValue::NativeRust => "native_rust",
    }
}
