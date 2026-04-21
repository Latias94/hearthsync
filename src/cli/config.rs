use super::ConfigCommands;
use super::app_support::{render_with_apply_target, render_with_value, stable_services};
use super::external_package::{
    build_analyze_external_package_request, build_apply_external_package_request,
    build_external_package_bundle_request, build_plan_external_package_request,
};
use super::output::config::{render_config_analysis, render_config_apply, render_config_plan};
use crate::core::error::AppResult;

pub(super) fn handle_config_command(json: bool, command: ConfigCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        ConfigCommands::Inspect { source } => render_with_value(
            json,
            || app.analyze_external_package(build_analyze_external_package_request(source)),
            render_config_analysis,
        )?,
        ConfigCommands::Plan {
            config_options,
            install_target,
            apply_mapping,
        } => render_with_apply_target(
            json,
            &app,
            install_target,
            apply_mapping,
            |target| {
                build_plan_external_package_request(
                    build_external_package_bundle_request(config_options.into()),
                    target.installation,
                    target.apply_mappings,
                )
            },
            |request| app.plan_external_package_apply(request),
            render_config_plan,
        )?,
        ConfigCommands::Apply {
            config_options,
            install_target,
            dry_run,
            backup_output,
            apply_mapping,
        } => render_with_apply_target(
            json,
            &app,
            install_target,
            apply_mapping,
            |target| {
                build_apply_external_package_request(
                    build_external_package_bundle_request(config_options.into()),
                    target.installation,
                    dry_run,
                    backup_output,
                    target.apply_mappings,
                )
            },
            |request| app.apply_external_package(request),
            render_config_apply,
        )?,
    }

    Ok(())
}
