use std::{fs, path::PathBuf};

use super::build_runtime;
use crate::cli::{AddonStateStorageArg, CliRuntimeArgs};
use crate::core::addon::AddonStateStorageKind;
use crate::core::app::{
    AddonProviderModeValue, AddonProviderOptionsValue, AddonProviderRetryPolicyValue,
    HttpNoValidatorCachePolicyValue, SetRuntimeSettingsAppRequest, StableAppServices,
    runtime_settings_path_guard,
};
use tempfile::tempdir;

#[test]
fn build_runtime_defaults_addon_state_storage_to_appdata() {
    let temp = tempdir().expect("temp dir");
    let _guard = runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
    let runtime = build_runtime(CliRuntimeArgs::default()).expect("build runtime");

    assert_eq!(
        runtime.addon_state_storage_kind(),
        AddonStateStorageKind::AppData
    );
    assert_eq!(
        runtime.capabilities().addon_provider,
        AddonProviderModeValue::ConfiguredDefault {
            options: AddonProviderOptionsValue::default(),
        }
    );
}

#[test]
fn build_runtime_applies_explicit_addon_state_storage_override() {
    let temp = tempdir().expect("temp dir");
    let _guard = runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
    let runtime = build_runtime(CliRuntimeArgs {
        addon_state_storage: Some(AddonStateStorageArg::Sidecar),
        ..CliRuntimeArgs::default()
    })
    .expect("build runtime");

    assert_eq!(
        runtime.addon_state_storage_kind(),
        AddonStateStorageKind::Sidecar
    );
}

#[test]
fn build_runtime_applies_addon_cache_overrides() {
    let temp = tempdir().expect("temp dir");
    let _guard = runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
    let runtime = build_runtime(CliRuntimeArgs {
        addon_cache_dir: Some(PathBuf::from("E:\\Cache")),
        addon_http_no_validator_window_secs: Some(120),
        ..CliRuntimeArgs::default()
    })
    .expect("build runtime");

    assert_eq!(
        runtime.capabilities().addon_provider,
        AddonProviderModeValue::ConfiguredDefault {
            options: AddonProviderOptionsValue {
                download_cache_dir: Some(PathBuf::from("E:\\Cache")),
                retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
                http_no_validator_cache_policy:
                    HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 120 },
            },
        }
    );
}

#[test]
fn build_runtime_resolves_relative_addon_cache_override_against_invocation_base() {
    let temp = tempdir().expect("temp dir");
    let _guard = runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
    let cwd = std::env::current_dir().expect("cwd");
    let runtime = build_runtime(CliRuntimeArgs {
        addon_cache_dir: Some(PathBuf::from("cache")),
        ..CliRuntimeArgs::default()
    })
    .expect("build runtime");

    assert_eq!(
        runtime.capabilities().addon_provider,
        AddonProviderModeValue::ConfiguredDefault {
            options: AddonProviderOptionsValue {
                download_cache_dir: Some(cwd.join("cache")),
                ..AddonProviderOptionsValue::default()
            },
        }
    );
}

#[test]
fn build_runtime_rejects_relative_persisted_addon_cache_dir() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    fs::create_dir_all(settings_path.parent().expect("settings dir")).expect("create settings dir");
    fs::write(&settings_path, "addon_cache_dir = \"cache\"").expect("write relative settings");

    let error =
        build_runtime(CliRuntimeArgs::default()).expect_err("relative persisted cache should fail");

    match error {
        crate::core::error::AppError::Validation(message) => {
            assert!(message.contains("persisted addon cache directory must be absolute"));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[test]
fn build_runtime_applies_always_refresh_override_for_no_validator_http_cache() {
    let temp = tempdir().expect("temp dir");
    let _guard = runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
    let runtime = build_runtime(CliRuntimeArgs {
        addon_http_no_validator_always_refresh: true,
        ..CliRuntimeArgs::default()
    })
    .expect("build runtime");

    assert_eq!(
        runtime.capabilities().addon_provider,
        AddonProviderModeValue::ConfiguredDefault {
            options: AddonProviderOptionsValue {
                http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue::AlwaysRefresh,
                ..AddonProviderOptionsValue::default()
            },
        }
    );
}

#[test]
fn build_runtime_applies_persisted_settings_before_cli_overrides() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    StableAppServices::new()
        .set_runtime_settings(SetRuntimeSettingsAppRequest {
            addon_state_storage: Some(crate::core::app::AddonStateStorageValue::Sidecar),
            clear_addon_state_storage: false,
            addon_cache_dir: Some(PathBuf::from("E:\\PersistedCache")),
            clear_addon_cache_dir: false,
            http_no_validator_cache_policy: Some(
                HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs: 600 },
            ),
            clear_http_no_validator_cache_policy: false,
        })
        .expect("seed settings");

    let runtime = build_runtime(CliRuntimeArgs {
        addon_http_no_validator_always_refresh: true,
        ..CliRuntimeArgs::default()
    })
    .expect("build runtime");

    assert_eq!(
        runtime.addon_state_storage_kind(),
        AddonStateStorageKind::Sidecar
    );
    assert_eq!(
        runtime.capabilities().addon_provider,
        AddonProviderModeValue::ConfiguredDefault {
            options: AddonProviderOptionsValue {
                download_cache_dir: Some(PathBuf::from("E:\\PersistedCache")),
                retry_policy: AddonProviderRetryPolicyValue { max_attempts: 1 },
                http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue::AlwaysRefresh,
            },
        }
    );
}

#[test]
fn build_runtime_fails_when_persisted_settings_file_is_invalid_toml() {
    let temp = tempdir().expect("temp dir");
    let settings_path = temp.path().join("settings").join("runtime.toml");
    let _guard = runtime_settings_path_guard(&settings_path);
    fs::create_dir_all(settings_path.parent().expect("settings dir")).expect("create settings dir");
    fs::write(&settings_path, "addon_state_storage = [").expect("write invalid settings");

    let error = build_runtime(CliRuntimeArgs::default()).expect_err("invalid settings should fail");

    match error {
        crate::core::error::AppError::Validation(message) => {
            assert!(message.contains("invalid runtime settings file"));
            assert!(message.contains(&settings_path.display().to_string()));
        }
        other => panic!("expected validation error, got {other:?}"),
    }
}
