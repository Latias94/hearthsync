use super::BundleCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::mapping::resolve_apply_mappings;
use super::output::{render, render_bundle_apply, render_bundle_apply_plan};
use crate::core::error::{AppError, AppResult};

mod request;

use request::{build_apply_bundle_request, build_plan_bundle_apply_request};

pub(super) fn handle_bundle_apply_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        BundleCommands::Plan {
            bundle,
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
            let plan = app.plan_bundle_apply(build_plan_bundle_apply_request(
                bundle,
                installation,
                apply_mappings,
            ))?;
            render(json, &plan, render_bundle_apply_plan)?;
        }
        BundleCommands::Unpack {
            bundle,
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
            let result = app.apply_bundle(build_apply_bundle_request(
                bundle,
                installation,
                dry_run,
                backup_output,
                apply_mappings,
            ))?;
            render(json, &result, render_bundle_apply)?;
        }
        _ => {
            return Err(AppError::Validation(
                "internal CLI routing error: bundle apply handler received unexpected command"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
