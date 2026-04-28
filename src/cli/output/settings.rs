use crate::core::app::{RuntimeSettingsInspectionResult, RuntimeSettingsMutationResult};

pub(in crate::cli) fn render_runtime_settings_inspection(
    item: &RuntimeSettingsInspectionResult,
) -> String {
    format!(
        "Settings file: {}\nSettings file exists: {}\nAddon state storage: {}\nAddon cache dir: {}\nHTTP no-validator cache policy: {}",
        item.settings_path.display(),
        item.settings_file_exists,
        item.settings
            .addon_state_storage
            .map(format_addon_state_storage)
            .unwrap_or("none"),
        item.settings
            .addon_cache_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        item.settings
            .http_no_validator_cache_policy
            .as_ref()
            .map(format_http_no_validator_cache_policy)
            .unwrap_or_else(|| "none".to_string()),
    )
}

pub(in crate::cli) fn render_runtime_settings_mutation(
    item: &RuntimeSettingsMutationResult,
) -> String {
    format!(
        "Settings file: {}\nSettings file exists: {}\nFile removed: {}\nAddon state storage: {}\nAddon cache dir: {}\nHTTP no-validator cache policy: {}",
        item.settings_path.display(),
        item.settings_file_exists,
        item.file_removed,
        item.settings
            .addon_state_storage
            .map(format_addon_state_storage)
            .unwrap_or("none"),
        item.settings
            .addon_cache_dir
            .as_deref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        item.settings
            .http_no_validator_cache_policy
            .as_ref()
            .map(format_http_no_validator_cache_policy)
            .unwrap_or_else(|| "none".to_string()),
    )
}

fn format_addon_state_storage(value: crate::core::app::AddonStateStorageValue) -> &'static str {
    match value {
        crate::core::app::AddonStateStorageValue::AppData => "app_data",
        crate::core::app::AddonStateStorageValue::Sidecar => "sidecar",
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{render_runtime_settings_inspection, render_runtime_settings_mutation};
    use crate::core::app::{
        AddonStateStorageValue, HttpNoValidatorCachePolicyValue, RuntimeSettingsInspectionResult,
        RuntimeSettingsMutationResult, RuntimeSettingsValue,
    };

    #[test]
    fn render_runtime_settings_inspection_reports_all_fields() {
        let rendered = render_runtime_settings_inspection(&RuntimeSettingsInspectionResult {
            settings_path: PathBuf::from("settings/runtime.toml"),
            settings_file_exists: true,
            settings: RuntimeSettingsValue {
                addon_state_storage: Some(AddonStateStorageValue::Sidecar),
                addon_cache_dir: Some(PathBuf::from("E:\\Cache")),
                http_no_validator_cache_policy: Some(
                    HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 120 },
                ),
            },
        });

        assert!(rendered.contains("Settings file: settings/runtime.toml"));
        assert!(rendered.contains("Settings file exists: true"));
        assert!(rendered.contains("Addon state storage: sidecar"));
        assert!(rendered.contains("Addon cache dir: E:\\Cache"));
        assert!(rendered.contains("HTTP no-validator cache policy: reuse_within_window(120s)"));
    }

    #[test]
    fn render_runtime_settings_mutation_reports_removed_file() {
        let rendered = render_runtime_settings_mutation(&RuntimeSettingsMutationResult {
            settings_path: PathBuf::from("settings/runtime.toml"),
            settings_file_exists: false,
            file_removed: true,
            settings: RuntimeSettingsValue::default(),
        });

        assert!(rendered.contains("Settings file exists: false"));
        assert!(rendered.contains("File removed: true"));
        assert!(rendered.contains("Addon cache dir: none"));
    }
}
