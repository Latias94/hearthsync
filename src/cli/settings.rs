use super::SettingsCommands;
use super::app_support::{render_with_value, stable_services};
use super::output::settings::{
    render_runtime_settings_inspection, render_runtime_settings_mutation,
};
use crate::core::app::{
    AddonCacheRepairRemotePolicyValue, AddonStateStorageValue, AppRuntime,
    HttpNoValidatorCachePolicyValue, SetRuntimeSettingsAppRequest,
};
use crate::core::error::AppResult;

pub(super) fn handle_settings_command(
    json: bool,
    runtime: AppRuntime,
    command: SettingsCommands,
) -> AppResult<()> {
    let app = stable_services(runtime);

    match command {
        SettingsCommands::Inspect => render_with_value(
            json,
            || app.inspect_runtime_settings(),
            render_runtime_settings_inspection,
        )?,
        SettingsCommands::Set {
            addon_state_storage,
            clear_addon_state_storage,
            addon_cache_dir,
            clear_addon_cache_dir,
            addon_http_no_validator_always_refresh,
            addon_http_no_validator_window_secs,
            clear_addon_http_no_validator_policy,
            addon_cache_repair_remote_policy,
            clear_addon_cache_repair_remote_policy,
        } => render_with_value(
            json,
            || {
                app.set_runtime_settings(SetRuntimeSettingsAppRequest {
                    addon_state_storage: addon_state_storage.map(addon_state_storage_value),
                    clear_addon_state_storage,
                    addon_cache_dir,
                    clear_addon_cache_dir,
                    http_no_validator_cache_policy: http_no_validator_cache_policy(
                        addon_http_no_validator_always_refresh,
                        addon_http_no_validator_window_secs,
                    ),
                    clear_http_no_validator_cache_policy: clear_addon_http_no_validator_policy,
                    addon_cache_repair_remote_policy: addon_cache_repair_remote_policy
                        .map(addon_cache_repair_remote_policy_value),
                    clear_addon_cache_repair_remote_policy,
                })
            },
            render_runtime_settings_mutation,
        )?,
        SettingsCommands::Reset => render_with_value(
            json,
            || app.reset_runtime_settings(),
            render_runtime_settings_mutation,
        )?,
    }

    Ok(())
}

fn addon_state_storage_value(value: crate::cli::AddonStateStorageArg) -> AddonStateStorageValue {
    match value {
        crate::cli::AddonStateStorageArg::AppData => AddonStateStorageValue::AppData,
        crate::cli::AddonStateStorageArg::Sidecar => AddonStateStorageValue::Sidecar,
    }
}

fn addon_cache_repair_remote_policy_value(
    value: crate::cli::AddonCacheRepairRemotePolicyArg,
) -> AddonCacheRepairRemotePolicyValue {
    value.into()
}

fn http_no_validator_cache_policy(
    always_refresh: bool,
    window_secs: Option<u64>,
) -> Option<HttpNoValidatorCachePolicyValue> {
    if always_refresh {
        return Some(HttpNoValidatorCachePolicyValue::AlwaysRefresh);
    }

    window_secs
        .map(|max_age_secs| HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs })
}
