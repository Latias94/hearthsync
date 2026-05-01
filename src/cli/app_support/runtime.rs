use std::path::PathBuf;

use crate::cli::CliRuntimeArgs;
use crate::core::app::{
    AddonProviderOptionsValue, AppRuntime, ExtendedAppServices, StableAppServices,
    load_persisted_runtime_settings_value,
};
use crate::core::error::{AppError, AppResult};

pub(in crate::cli) fn build_runtime(options: CliRuntimeArgs) -> AppResult<AppRuntime> {
    let persisted_settings = load_persisted_runtime_settings_value()?.unwrap_or_default();
    let relative_path_base = std::env::current_dir()?;
    let download_cache_dir = match options.addon_cache_dir.clone() {
        Some(path) => Some(path),
        None => persisted_settings
            .addon_cache_dir
            .clone()
            .map(|path| validate_persisted_runtime_path(path, "addon cache directory"))
            .transpose()?,
    };
    let provider_options = AddonProviderOptionsValue {
        download_cache_dir,
        http_no_validator_cache_policy: options
            .http_no_validator_cache_policy()
            .or(persisted_settings.http_no_validator_cache_policy.clone())
            .unwrap_or_default(),
        ..AddonProviderOptionsValue::default()
    };

    let mut runtime = AppRuntime::builder()
        .with_relative_path_base(Some(relative_path_base))
        .with_addon_provider_options(provider_options);

    if let Some(storage) = persisted_settings.addon_state_storage {
        runtime = runtime.with_addon_state_storage_kind(storage.into_domain());
    }

    if let Some(storage) = options.addon_state_storage {
        runtime = runtime.with_addon_state_storage_kind(storage.into());
    }

    runtime.build()
}

fn validate_persisted_runtime_path(path: PathBuf, description: &str) -> AppResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    Err(AppError::Validation(format!(
        "persisted {description} must be absolute: {}",
        path.display()
    )))
}

pub(in crate::cli) fn stable_services(runtime: AppRuntime) -> StableAppServices {
    StableAppServices::with_runtime(runtime)
}

pub(in crate::cli) fn extended_services(runtime: AppRuntime) -> ExtendedAppServices {
    ExtendedAppServices::with_runtime(runtime)
}
