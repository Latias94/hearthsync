use std::path::PathBuf;

use tempfile::tempdir;

use super::{RuntimeSettingsService, runtime_settings_path_guard};
use crate::core::app::{
    AddonCacheRepairRemotePolicyValue, AddonStateStorageValue, AppRuntime,
    HttpNoValidatorCachePolicyValue, SetRuntimeSettingsAppRequest,
};
use crate::core::error::AppError;

#[test]
fn runtime_settings_service_roundtrips_settings_file() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());

    let mutation = service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: Some(AddonStateStorageValue::Sidecar),
            clear_addon_state_storage: false,
            addon_cache_dir: Some(PathBuf::from("E:/Cache")),
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: Some(
                HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 120 },
            ),
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: Some(
                AddonCacheRepairRemotePolicyValue::RequireRemote,
            ),
            clear_addon_cache_repair_remote_policy: false,
            addon_search_cache_ttl_secs: Some(60),
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect("set settings");
    let inspection = service.inspect().expect("inspect settings");

    assert!(!mutation.file_removed);
    assert!(mutation.settings_file_exists);
    assert_eq!(
        mutation.settings.addon_state_storage,
        Some(AddonStateStorageValue::Sidecar)
    );
    assert_eq!(
        mutation.settings.addon_cache_repair_remote_policy,
        Some(AddonCacheRepairRemotePolicyValue::RequireRemote)
    );
    assert_eq!(mutation.settings.addon_search_cache_ttl_secs, Some(60));
    assert!(settings_path.is_file());
    assert!(inspection.settings_file_exists);
    assert_eq!(inspection.settings, mutation.settings);
}

#[test]
fn runtime_settings_service_resolves_relative_addon_cache_dir_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(temp.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );

    let mutation = service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: None,
            clear_addon_state_storage: false,
            addon_cache_dir: Some(PathBuf::from("cache")),
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: None,
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: None,
            clear_addon_cache_repair_remote_policy: false,
            addon_search_cache_ttl_secs: None,
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect("set relative cache dir");
    let inspection = service.inspect().expect("inspect settings");

    assert_eq!(
        mutation.settings.addon_cache_dir,
        Some(temp.path().join("cache"))
    );
    assert_eq!(inspection.settings, mutation.settings);
}

#[test]
fn runtime_settings_service_rejects_relative_addon_cache_dir_without_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());

    let error = service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: None,
            clear_addon_state_storage: false,
            addon_cache_dir: Some(PathBuf::from("cache")),
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: None,
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: None,
            clear_addon_cache_repair_remote_policy: false,
            addon_search_cache_ttl_secs: None,
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect_err("relative cache dir should fail closed");

    assert!(
        error
            .to_string()
            .contains("addon cache directory relative path requires")
    );
}

#[test]
fn runtime_settings_service_reset_removes_settings_file() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());

    service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: None,
            clear_addon_state_storage: false,
            addon_cache_dir: Some(PathBuf::from("E:/Cache")),
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: None,
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: None,
            clear_addon_cache_repair_remote_policy: false,
            addon_search_cache_ttl_secs: None,
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect("seed settings");
    let mutation = service.reset().expect("reset settings");

    assert!(mutation.file_removed);
    assert!(!mutation.settings_file_exists);
    assert!(!settings_path.exists());
    assert_eq!(
        mutation.settings,
        crate::core::app::RuntimeSettingsValue::default()
    );
}

#[test]
fn runtime_settings_service_rejects_empty_mutation() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());

    let error = service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: None,
            clear_addon_state_storage: false,
            addon_cache_dir: None,
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: None,
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: None,
            clear_addon_cache_repair_remote_policy: false,
            addon_search_cache_ttl_secs: None,
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect_err("empty mutation should fail");

    assert!(matches!(error, AppError::Validation(_)));
}

#[test]
fn runtime_settings_service_rejects_set_and_clear_same_field() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());

    let error = service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: Some(AddonStateStorageValue::Sidecar),
            clear_addon_state_storage: true,
            addon_cache_dir: None,
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: None,
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: None,
            clear_addon_cache_repair_remote_policy: false,
            addon_search_cache_ttl_secs: None,
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect_err("conflicting mutation should fail");

    assert!(matches!(error, AppError::Validation(_)));
}

#[test]
fn runtime_settings_service_rejects_set_and_clear_remote_repair_policy() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());

    let error = service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: None,
            clear_addon_state_storage: false,
            addon_cache_dir: None,
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: None,
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: Some(AddonCacheRepairRemotePolicyValue::LocalOnly),
            clear_addon_cache_repair_remote_policy: true,
            addon_search_cache_ttl_secs: None,
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect_err("conflicting remote repair policy mutation should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(
        error
            .to_string()
            .contains("cannot set and clear addon_cache_repair_remote_policy")
    );
}

#[test]
fn runtime_settings_service_reports_path_for_invalid_settings_file() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());
    std::fs::create_dir_all(settings_path.parent().expect("settings dir"))
        .expect("create settings dir");
    std::fs::write(&settings_path, "addon_cache_dir = [").expect("write invalid settings");

    let error = service
        .inspect()
        .expect_err("invalid settings should fail inspection");

    match error {
        AppError::Validation(message) => {
            assert!(message.contains("invalid runtime settings file"));
            assert!(message.contains(&settings_path.display().to_string()));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn runtime_settings_service_rejects_invalid_persisted_setting_contracts() {
    for (case_name, content, expected_message) in [
        (
            "relative cache dir",
            "addon_cache_dir = \"cache\"",
            "persisted addon cache directory must be absolute",
        ),
        (
            "zero no-validator window",
            "[http_no_validator_cache_policy]\nmode = \"reuse_within_window\"\nmax_age_secs = 0\n",
            "HTTP no-validator cache window must be greater than zero seconds",
        ),
    ] {
        let temp = tempdir().expect("temp dir");
        let settings_path = temp.path().join("settings").join("runtime.toml");
        let _guard = runtime_settings_path_guard(&settings_path);
        let service = RuntimeSettingsService::with_runtime(AppRuntime::new());
        std::fs::create_dir_all(settings_path.parent().expect("settings dir"))
            .expect("create settings dir");
        std::fs::write(&settings_path, content).expect("write settings");

        let error = service.inspect().expect_err(case_name);

        match error {
            AppError::Validation(message) => {
                assert!(message.contains("invalid runtime settings file"));
                assert!(message.contains(&settings_path.display().to_string()));
                assert!(
                    message.contains(expected_message),
                    "{case_name}: expected `{expected_message}`, got `{message}`"
                );
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }
}

#[test]
fn runtime_settings_service_rejects_invalid_http_cache_policy_mutation() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    let service = RuntimeSettingsService::with_runtime(AppRuntime::new());

    let error = service
        .set(SetRuntimeSettingsAppRequest {
            addon_state_storage: None,
            clear_addon_state_storage: false,
            addon_cache_dir: None,
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: Some(
                HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 0 },
            ),
            clear_http_no_validator_cache_policy: false,
            addon_cache_repair_remote_policy: None,
            clear_addon_cache_repair_remote_policy: false,
            addon_search_cache_ttl_secs: None,
            clear_addon_search_cache_ttl_secs: false,
        })
        .expect_err("zero cache window should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(
        error
            .to_string()
            .contains("HTTP no-validator cache window must be greater than zero seconds")
    );
    assert!(!settings_path.exists());
}
