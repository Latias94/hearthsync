use super::ExternalPackageCommands;
use super::app_support::{resolve_cli_apply_target, stable_services};
use super::output::{
    render, render_external_package_analysis, render_external_package_apply,
    render_external_package_plan,
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
        ExternalPackageCommands::Inspect { source } => {
            let analysis =
                app.analyze_external_package(build_analyze_external_package_request(source))?;
            render(json, &analysis, render_external_package_analysis)?;
        }
        ExternalPackageCommands::Plan {
            bundle_options,
            install_target,
            apply_mapping,
        } => {
            let target = resolve_cli_apply_target(&app, install_target, apply_mapping)?;
            let external_package = build_external_package_bundle_request(bundle_options);
            let plan = app.plan_external_package_apply(build_plan_external_package_request(
                external_package,
                target.installation,
                target.apply_mappings,
            ))?;
            render(json, &plan, render_external_package_plan)?;
        }
        ExternalPackageCommands::Apply {
            bundle_options,
            install_target,
            dry_run,
            backup_output,
            apply_mapping,
        } => {
            let target = resolve_cli_apply_target(&app, install_target, apply_mapping)?;
            let external_package = build_external_package_bundle_request(bundle_options);
            let result = app.apply_external_package(build_apply_external_package_request(
                external_package,
                target.installation,
                dry_run,
                backup_output,
                target.apply_mappings,
            ))?;
            render(json, &result, render_external_package_apply)?;
        }
    }

    Ok(())
}
