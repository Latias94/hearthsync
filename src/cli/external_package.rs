use super::ExternalPackageCommands;
use super::app_support::{render_with_apply_target, render_with_value, stable_services};
use super::output::{
    render_external_package_analysis, render_external_package_apply, render_external_package_plan,
};
use crate::core::error::AppResult;

mod request;

use request::{
    build_analyze_external_package_request, build_apply_external_package_request,
    build_external_package_bundle_request, build_plan_external_package_request,
};

pub(super) fn handle_external_package_command(
    json: bool,
    command: ExternalPackageCommands,
) -> AppResult<()> {
    let app = stable_services();

    match command {
        ExternalPackageCommands::Inspect { source } => render_with_value(
            json,
            || app.analyze_external_package(build_analyze_external_package_request(source)),
            render_external_package_analysis,
        )?,
        ExternalPackageCommands::Plan {
            bundle_options,
            install_target,
            apply_mapping,
        } => render_with_apply_target(
            json,
            &app,
            install_target,
            apply_mapping,
            |target| {
                build_plan_external_package_request(
                    build_external_package_bundle_request(bundle_options),
                    target.installation,
                    target.apply_mappings,
                )
            },
            |request| app.plan_external_package_apply(request),
            render_external_package_plan,
        )?,
        ExternalPackageCommands::Apply {
            bundle_options,
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
                    build_external_package_bundle_request(bundle_options),
                    target.installation,
                    dry_run,
                    backup_output,
                    target.apply_mappings,
                )
            },
            |request| app.apply_external_package(request),
            render_external_package_apply,
        )?,
    }

    Ok(())
}
