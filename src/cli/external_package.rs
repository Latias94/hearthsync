use super::ExternalPackageCommands;
use super::app_support::{
    CliAppContext, render_task_result, render_with_apply_target_task_result, stable_services,
};
use super::output::external_package::{
    render_external_package_analysis, render_external_package_apply, render_external_package_plan,
};
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

mod request;

pub(in crate::cli) use request::{
    build_analyze_external_package_request, build_apply_external_package_request,
    build_external_package_bundle_request, build_plan_external_package_request,
};

pub(super) fn handle_external_package_command(
    json: bool,
    runtime: AppRuntime,
    command: ExternalPackageCommands,
) -> AppResult<()> {
    let app = stable_services(runtime.clone());

    match command {
        ExternalPackageCommands::Inspect { source } => {
            render_task_result(
                json,
                || app.analyze_external_package(build_analyze_external_package_request(source)),
                render_external_package_analysis,
            )?;
        }
        ExternalPackageCommands::Plan {
            bundle_options,
            install_target,
            apply_mapping,
        } => {
            render_with_apply_target_task_result(
                json,
                CliAppContext::new(&app, &runtime),
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
            )?;
        }
        ExternalPackageCommands::Apply {
            bundle_options,
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
            )?;
        }
    }

    Ok(())
}
