use super::ExternalPackageCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::mapping::resolve_apply_mappings;
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
            install,
            flavor,
            mapping_file,
            target_account,
            target_server,
            target_character,
            selected_accounts,
            all_accounts,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let apply_mappings = resolve_apply_mappings(
                mapping_file.as_deref(),
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            )?;
            let external_package = build_external_package_bundle_request(bundle_options);
            let plan = app.plan_external_package_apply(build_plan_external_package_request(
                external_package,
                installation,
                apply_mappings,
            ))?;
            render(json, &plan, render_external_package_plan)?;
        }
        ExternalPackageCommands::Apply {
            bundle_options,
            install,
            flavor,
            dry_run,
            backup_output,
            mapping_file,
            target_account,
            target_server,
            target_character,
            selected_accounts,
            all_accounts,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let apply_mappings = resolve_apply_mappings(
                mapping_file.as_deref(),
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            )?;
            let external_package = build_external_package_bundle_request(bundle_options);
            let result = app.apply_external_package(build_apply_external_package_request(
                external_package,
                installation,
                dry_run,
                backup_output,
                apply_mappings,
            ))?;
            render(json, &result, render_external_package_apply)?;
        }
    }

    Ok(())
}
