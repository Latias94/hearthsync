use super::ExternalPackageCommands;
use super::app_support::{
    CliAppContext, render_task_result, render_with_apply_target_task_result, render_with_value,
    stable_services,
};
use super::output::external_package::{
    render_external_package_analysis, render_external_package_apply,
    render_external_package_bundle, render_external_package_plan,
};
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

mod request;

pub(in crate::cli) use request::{
    build_analyze_external_package_request, build_apply_external_package_request,
    build_external_package_bundle_export_request, build_external_package_bundle_request,
    build_plan_external_package_request,
};

pub(super) fn handle_external_package_command(
    json: bool,
    runtime: AppRuntime,
    command: ExternalPackageCommands,
) -> AppResult<()> {
    let app = stable_services(runtime.clone());

    match command {
        ExternalPackageCommands::Inspect {
            source,
            source_layout,
        } => {
            render_task_result(
                json,
                || {
                    app.analyze_external_package(build_analyze_external_package_request(
                        source,
                        source_layout,
                    ))
                },
                render_external_package_analysis,
            )?;
        }
        ExternalPackageCommands::Bundle {
            bundle_options,
            output,
            sharing_mode,
            allow_public_sharing_risks,
            excluded_wtf_scopes,
        } => {
            render_with_value(
                json,
                || {
                    let handle = app.create_external_package_bundle(
                        build_external_package_bundle_export_request(
                            bundle_options,
                            Some(output),
                            sharing_mode,
                            allow_public_sharing_risks,
                            excluded_wtf_scopes,
                        ),
                    )?;
                    Ok(handle.as_ref().clone())
                },
                render_external_package_bundle,
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
