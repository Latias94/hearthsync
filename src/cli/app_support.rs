use super::mapping::resolve_apply_mappings;
use super::output::render;
use super::{ApplyMappingArgs, CliRuntimeArgs, InstallTargetArgs, OptionalInstallTargetArgs};
use crate::core::app::{
    AddonProviderOptionsValue, AppRuntime, BundleApplyMappingsValue, ExtendedAppServices,
    ResolveInstallationRequest, ResolvedInstallationValue, StableAppServices, TaskRun,
    load_persisted_runtime_settings_value,
};
use crate::core::error::AppResult;
use serde::Serialize;

pub(super) fn build_runtime(options: CliRuntimeArgs) -> AppResult<AppRuntime> {
    let persisted_settings = load_persisted_runtime_settings_value()?.unwrap_or_default();
    let provider_options = AddonProviderOptionsValue {
        download_cache_dir: options
            .addon_cache_dir
            .clone()
            .or(persisted_settings.addon_cache_dir.clone()),
        http_no_validator_cache_policy: options
            .http_no_validator_cache_policy()
            .or(persisted_settings.http_no_validator_cache_policy.clone())
            .unwrap_or_default(),
        ..AddonProviderOptionsValue::default()
    };

    let mut runtime = AppRuntime::with_addon_provider_options(provider_options)
        .with_relative_path_base(Some(std::env::current_dir()?));

    if let Some(storage) = persisted_settings.addon_state_storage {
        runtime = runtime.with_addon_state_storage_kind(storage.into_domain());
    }

    if let Some(storage) = options.addon_state_storage {
        runtime = runtime.with_addon_state_storage_kind(storage.into());
    }

    Ok(runtime)
}

pub(super) fn stable_services(runtime: AppRuntime) -> StableAppServices {
    StableAppServices::with_runtime(runtime)
}

pub(super) fn extended_services(runtime: AppRuntime) -> ExtendedAppServices {
    ExtendedAppServices::with_runtime(runtime)
}

pub(super) struct ResolvedCliApplyTarget {
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

#[derive(Clone, Copy)]
pub(super) struct CliAppContext<'a> {
    services: &'a StableAppServices,
    runtime: &'a AppRuntime,
}

impl<'a> CliAppContext<'a> {
    pub(super) fn new(services: &'a StableAppServices, runtime: &'a AppRuntime) -> Self {
        Self { services, runtime }
    }
}

pub(super) fn resolve_cli_installation(
    services: &StableAppServices,
    install_target: InstallTargetArgs,
) -> AppResult<ResolvedInstallationValue> {
    services.resolve_installation(ResolveInstallationRequest {
        path: install_target.install,
        flavor: install_target.flavor.map(Into::into),
    })
}

pub(super) fn resolve_optional_cli_installation(
    services: &StableAppServices,
    install_target: OptionalInstallTargetArgs,
) -> AppResult<Option<ResolvedInstallationValue>> {
    let Some(path) = install_target.install else {
        return Ok(None);
    };

    services
        .resolve_installation(ResolveInstallationRequest {
            path,
            flavor: install_target.flavor.map(Into::into),
        })
        .map(Some)
}

pub(super) fn resolve_cli_apply_target(
    context: CliAppContext<'_>,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
) -> AppResult<ResolvedCliApplyTarget> {
    Ok(ResolvedCliApplyTarget {
        installation: resolve_cli_installation(context.services, install_target)?,
        apply_mappings: resolve_apply_mappings(apply_mapping, context.runtime)?,
    })
}

pub(super) fn render_with_installation<Req, Res, Build, Invoke, Format>(
    json: bool,
    services: &StableAppServices,
    install_target: InstallTargetArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Build: FnOnce(ResolvedInstallationValue) -> Req,
    Invoke: FnOnce(Req) -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let installation = resolve_cli_installation(services, install_target)?;
    let result = invoke(build_request(installation))?;
    render(json, &result, text_renderer)
}

pub(super) fn render_with_fallible_installation<Req, Res, Build, Invoke, Format>(
    json: bool,
    services: &StableAppServices,
    install_target: InstallTargetArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Build: FnOnce(ResolvedInstallationValue) -> AppResult<Req>,
    Invoke: FnOnce(Req) -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let installation = resolve_cli_installation(services, install_target)?;
    let request = build_request(installation)?;
    let result = invoke(request)?;
    render(json, &result, text_renderer)
}

pub(super) fn render_with_value<Res, Invoke, Format>(
    json: bool,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Invoke: FnOnce() -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let result = invoke()?;
    render(json, &result, text_renderer)
}

pub(super) fn render_task_result<Res, Invoke, Format>(
    json: bool,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<TaskRun<Res>>
where
    Res: Serialize,
    Invoke: FnOnce() -> AppResult<TaskRun<Res>>,
    Format: FnOnce(&Res) -> String,
{
    let run = invoke()?;
    render(json, &run.result, text_renderer)?;
    Ok(run)
}

pub(super) fn render_with_installation_task_result<Req, Res, Build, Invoke, Format>(
    json: bool,
    services: &StableAppServices,
    install_target: InstallTargetArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<TaskRun<Res>>
where
    Res: Serialize,
    Build: FnOnce(ResolvedInstallationValue) -> Req,
    Invoke: FnOnce(Req) -> AppResult<TaskRun<Res>>,
    Format: FnOnce(&Res) -> String,
{
    let installation = resolve_cli_installation(services, install_target)?;
    render_task_result(json, || invoke(build_request(installation)), text_renderer)
}

pub(super) fn render_with_apply_target<Req, Res, Build, Invoke, Format>(
    json: bool,
    context: CliAppContext<'_>,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Build: FnOnce(ResolvedCliApplyTarget) -> Req,
    Invoke: FnOnce(Req) -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let target = resolve_cli_apply_target(context, install_target, apply_mapping)?;
    let result = invoke(build_request(target))?;
    render(json, &result, text_renderer)
}

pub(super) fn render_with_apply_target_task_result<Req, Res, Build, Invoke, Format>(
    json: bool,
    context: CliAppContext<'_>,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<TaskRun<Res>>
where
    Res: Serialize,
    Build: FnOnce(ResolvedCliApplyTarget) -> Req,
    Invoke: FnOnce(Req) -> AppResult<TaskRun<Res>>,
    Format: FnOnce(&Res) -> String,
{
    let target = resolve_cli_apply_target(context, install_target, apply_mapping)?;
    render_task_result(json, || invoke(build_request(target)), text_renderer)
}

#[cfg(test)]
mod tests {
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
        let _guard =
            runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
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
        let _guard =
            runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
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
        let _guard =
            runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
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
    fn build_runtime_applies_always_refresh_override_for_no_validator_http_cache() {
        let temp = tempdir().expect("temp dir");
        let _guard =
            runtime_settings_path_guard(&temp.path().join("settings").join("runtime.toml"));
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
        fs::create_dir_all(settings_path.parent().expect("settings dir"))
            .expect("create settings dir");
        fs::write(&settings_path, "addon_state_storage = [").expect("write invalid settings");

        let error =
            build_runtime(CliRuntimeArgs::default()).expect_err("invalid settings should fail");

        match error {
            crate::core::error::AppError::Validation(message) => {
                assert!(message.contains("invalid runtime settings file"));
                assert!(message.contains(&settings_path.display().to_string()));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }
}
