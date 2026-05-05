use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
use std::ffi::OsString;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};

use crate::core::app::{
    AppRuntime, RuntimeSettingsInspectionResult, RuntimeSettingsMutationResult,
    RuntimeSettingsValue, SetRuntimeSettingsAppRequest,
};
use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::{AppError, AppResult};
use crate::core::platform_dirs::app_data_subdir;

const RUNTIME_SETTINGS_RELATIVE_PATH: &str = "settings/runtime.toml";
#[cfg(test)]
const TEST_RUNTIME_SETTINGS_PATH_ENV: &str = "HEARTHSYNC_TEST_RUNTIME_SETTINGS_PATH";

#[derive(Debug, Clone, Default)]
pub(super) struct RuntimeSettingsService {
    runtime: AppRuntime,
}

impl RuntimeSettingsService {
    pub(super) fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub(super) fn inspect(&self) -> AppResult<RuntimeSettingsInspectionResult> {
        let settings_path = runtime_settings_path()?;
        let settings = load_persisted_runtime_settings_value()?;

        Ok(RuntimeSettingsInspectionResult {
            settings_path,
            settings_file_exists: settings.is_some(),
            settings: settings.unwrap_or_default(),
        })
    }

    pub(super) fn set(
        &self,
        request: SetRuntimeSettingsAppRequest,
    ) -> AppResult<RuntimeSettingsMutationResult> {
        validate_set_runtime_settings_request(&request)?;

        let settings_path = runtime_settings_path()?;
        let mut settings = load_persisted_runtime_settings_value()?.unwrap_or_default();
        let file_existed = settings_path.is_file();

        if request.clear_addon_state_storage {
            settings.addon_state_storage = None;
        }
        if let Some(value) = request.addon_state_storage {
            settings.addon_state_storage = Some(value);
        }
        if request.clear_addon_cache_dir {
            settings.addon_cache_dir = None;
        }
        if let Some(value) = request.addon_cache_dir {
            settings.addon_cache_dir = Some(
                self.runtime
                    .resolve_output_path(value, "addon cache directory")?,
            );
        }
        if request.clear_http_no_validator_cache_policy {
            settings.http_no_validator_cache_policy = None;
        }
        if let Some(value) = request.http_no_validator_cache_policy {
            settings.http_no_validator_cache_policy = Some(value);
        }
        if request.clear_addon_cache_repair_remote_policy {
            settings.addon_cache_repair_remote_policy = None;
        }
        if let Some(value) = request.addon_cache_repair_remote_policy {
            settings.addon_cache_repair_remote_policy = Some(value);
        }

        let settings_file_exists = save_persisted_runtime_settings_value(&settings)?;

        Ok(RuntimeSettingsMutationResult {
            settings_path,
            settings_file_exists,
            file_removed: file_existed && !settings_file_exists,
            settings: if settings_file_exists {
                settings
            } else {
                RuntimeSettingsValue::default()
            },
        })
    }

    pub(super) fn reset(&self) -> AppResult<RuntimeSettingsMutationResult> {
        let settings_path = runtime_settings_path()?;
        let file_removed = remove_persisted_runtime_settings_file()?;

        Ok(RuntimeSettingsMutationResult {
            settings_path,
            settings_file_exists: false,
            file_removed,
            settings: RuntimeSettingsValue::default(),
        })
    }
}

pub(crate) fn load_persisted_runtime_settings_value() -> AppResult<Option<RuntimeSettingsValue>> {
    let settings_path = runtime_settings_path()?;
    if !settings_path.is_file() {
        return Ok(None);
    }

    let content = fs::read_to_string(&settings_path)?;
    let settings = toml::from_str::<RuntimeSettingsValue>(&content).map_err(|error| {
        AppError::Validation(format!(
            "invalid runtime settings file `{}`: {error}",
            settings_path.display()
        ))
    })?;
    validate_runtime_settings_value(&settings).map_err(|error| {
        AppError::Validation(format!(
            "invalid runtime settings file `{}`: {error}",
            settings_path.display()
        ))
    })?;
    Ok(Some(settings))
}

fn save_persisted_runtime_settings_value(settings: &RuntimeSettingsValue) -> AppResult<bool> {
    let settings_path = runtime_settings_path()?;
    if settings.is_empty() {
        if settings_path.is_file() {
            fs::remove_file(&settings_path)?;
        }
        remove_empty_runtime_settings_parent_dirs(&settings_path)?;
        return Ok(false);
    }

    validate_runtime_settings_value(settings)?;
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(settings)?;
    write_bytes_atomically(&settings_path, content.as_bytes())?;
    Ok(true)
}

fn remove_persisted_runtime_settings_file() -> AppResult<bool> {
    let settings_path = runtime_settings_path()?;
    if !settings_path.is_file() {
        return Ok(false);
    }

    fs::remove_file(&settings_path)?;
    remove_empty_runtime_settings_parent_dirs(&settings_path)?;
    Ok(true)
}

fn validate_set_runtime_settings_request(request: &SetRuntimeSettingsAppRequest) -> AppResult<()> {
    if request.clear_addon_state_storage && request.addon_state_storage.is_some() {
        return Err(AppError::Validation(
            "cannot set and clear addon_state_storage in the same settings mutation".to_string(),
        ));
    }
    if request.clear_addon_cache_dir && request.addon_cache_dir.is_some() {
        return Err(AppError::Validation(
            "cannot set and clear addon_cache_dir in the same settings mutation".to_string(),
        ));
    }
    if request.clear_http_no_validator_cache_policy
        && request.http_no_validator_cache_policy.is_some()
    {
        return Err(AppError::Validation(
            "cannot set and clear http_no_validator_cache_policy in the same settings mutation"
                .to_string(),
        ));
    }
    if request.clear_addon_cache_repair_remote_policy
        && request.addon_cache_repair_remote_policy.is_some()
    {
        return Err(AppError::Validation(
            "cannot set and clear addon_cache_repair_remote_policy in the same settings mutation"
                .to_string(),
        ));
    }
    if request.addon_state_storage.is_none()
        && !request.clear_addon_state_storage
        && request.addon_cache_dir.is_none()
        && !request.clear_addon_cache_dir
        && request.http_no_validator_cache_policy.is_none()
        && !request.clear_http_no_validator_cache_policy
        && request.addon_cache_repair_remote_policy.is_none()
        && !request.clear_addon_cache_repair_remote_policy
    {
        return Err(AppError::Validation(
            "settings mutation must change at least one field".to_string(),
        ));
    }

    Ok(())
}

fn validate_runtime_settings_value(settings: &RuntimeSettingsValue) -> AppResult<()> {
    if let Some(path) = &settings.addon_cache_dir
        && !path.is_absolute()
    {
        return Err(AppError::Validation(format!(
            "persisted addon cache directory must be absolute: {}",
            path.display()
        )));
    }

    if let Some(policy) = &settings.http_no_validator_cache_policy {
        policy.clone().into_domain()?;
    }

    Ok(())
}

fn runtime_settings_path() -> AppResult<PathBuf> {
    #[cfg(test)]
    if let Some(path) = std::env::var_os(TEST_RUNTIME_SETTINGS_PATH_ENV) {
        return Ok(PathBuf::from(path));
    }

    app_data_subdir(Path::new(RUNTIME_SETTINGS_RELATIVE_PATH))
}

fn remove_empty_runtime_settings_parent_dirs(settings_path: &Path) -> AppResult<()> {
    let Some(settings_dir) = settings_path.parent() else {
        return Ok(());
    };
    if settings_dir.exists() && fs::read_dir(settings_dir)?.next().is_none() {
        fs::remove_dir(settings_dir)?;
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn runtime_settings_path_guard(path: &Path) -> RuntimeSettingsPathGuard {
    static RUNTIME_SETTINGS_PATH_ENV_MUTEX: Mutex<()> = Mutex::new(());

    let lock = RUNTIME_SETTINGS_PATH_ENV_MUTEX
        .lock()
        .expect("runtime settings env lock");
    let previous = std::env::var_os(TEST_RUNTIME_SETTINGS_PATH_ENV);
    unsafe {
        std::env::set_var(TEST_RUNTIME_SETTINGS_PATH_ENV, path);
    }

    RuntimeSettingsPathGuard {
        previous,
        _lock: lock,
    }
}

#[cfg(test)]
pub(crate) struct RuntimeSettingsPathGuard {
    previous: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for RuntimeSettingsPathGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => unsafe {
                std::env::set_var(TEST_RUNTIME_SETTINGS_PATH_ENV, value);
            },
            None => unsafe {
                std::env::remove_var(TEST_RUNTIME_SETTINGS_PATH_ENV);
            },
        }
    }
}

#[cfg(test)]
mod tests;
