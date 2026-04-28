use super::ConfigCommands;
use super::app_support::{
    CliAppContext, render_task_result, render_with_apply_target_task_result, stable_services,
};
use super::output::config::{render_config_analysis, render_config_apply, render_config_plan};
use crate::core::app::{
    AppRuntime, ApplyConfigAppRequest, BundleApplyDefaultsValue, ConfigPackageAppRequest,
    InspectConfigAppRequest, PlanConfigApplyAppRequest, ResourceApplyPolicyValue,
};
use crate::core::error::AppResult;

pub(super) fn handle_config_command(
    json: bool,
    runtime: AppRuntime,
    command: ConfigCommands,
) -> AppResult<()> {
    let app = stable_services(runtime.clone());

    match command {
        ConfigCommands::Inspect { source } => {
            render_task_result(
                json,
                || {
                    app.inspect_config(InspectConfigAppRequest {
                        source_path: source,
                    })
                },
                render_config_analysis,
            )?;
        }
        ConfigCommands::Plan {
            config_options,
            install_target,
            apply_mapping,
        } => {
            render_with_apply_target_task_result(
                json,
                CliAppContext::new(&app, &runtime),
                install_target,
                apply_mapping,
                |target| PlanConfigApplyAppRequest {
                    config_package: build_config_package_request(config_options),
                    installation: target.installation,
                    apply_mappings: target.apply_mappings,
                },
                |request| app.plan_config_apply(request),
                render_config_plan,
            )?;
        }
        ConfigCommands::Apply {
            config_options,
            install_target,
            dry_run,
            backup_output,
            apply_mapping,
        } => {
            render_with_apply_target_task_result(
                json,
                CliAppContext::new(&app, &runtime),
                install_target,
                apply_mapping,
                |target| ApplyConfigAppRequest {
                    config_package: build_config_package_request(config_options),
                    installation: target.installation,
                    dry_run,
                    backup_output_path: backup_output,
                    apply_mappings: target.apply_mappings,
                },
                |request| app.apply_config(request),
                render_config_apply,
            )?;
        }
    }

    Ok(())
}

fn build_config_package_request(
    options: super::args::config::ConfigPackageOptions,
) -> ConfigPackageAppRequest {
    let apply_defaults = build_config_apply_defaults(&options);

    ConfigPackageAppRequest {
        source_path: options.source,
        source_flavor: options.source_flavor.into(),
        source_platform: options.source_platform.map(Into::into),
        supported_targets: if options.supported_targets.is_empty() {
            Vec::new()
        } else {
            options
                .supported_targets
                .into_iter()
                .map(Into::into)
                .collect()
        },
        output_path: None,
        package_id: options.package_id,
        package_name: options.package_name,
        created_by: options.created_by,
        description: options.description,
        apply_defaults,
    }
}

fn build_config_apply_defaults(
    options: &super::args::config::ConfigPackageOptions,
) -> Option<BundleApplyDefaultsValue> {
    let has_override = options.no_backup
        || options.addons_policy.is_some()
        || options.wtf_common_policy.is_some()
        || options.wtf_characters_policy.is_some()
        || options.fonts_policy.is_some()
        || options.interface_assets_policy.is_some();

    if !has_override {
        return None;
    }

    let mut defaults = BundleApplyDefaultsValue::author_package_defaults();
    defaults.create_backup = !options.no_backup;
    if let Some(policy) = options.addons_policy {
        defaults.addons = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.wtf_common_policy {
        defaults.wtf_common = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.wtf_characters_policy {
        defaults.wtf_characters = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.fonts_policy {
        defaults.fonts = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }
    if let Some(policy) = options.interface_assets_policy {
        defaults.interface_assets = ResourceApplyPolicyValue::from_domain(
            crate::core::manifest::ResourceApplyPolicy::from(policy),
        );
    }

    Some(defaults)
}
