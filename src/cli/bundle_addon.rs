use super::BundleCommands;
use super::app_support::{extended_services, resolve_cli_installation};
use super::output::{render, render_bundle_addon_lock_apply, render_bundle_addon_lock_plan};
use crate::core::error::{AppError, AppResult};

mod request;

use request::{build_apply_bundle_addon_lock_request, build_plan_bundle_addon_lock_request};

pub(super) fn handle_bundle_addon_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = extended_services();

    match command {
        BundleCommands::AddonPlan {
            bundle,
            install,
            flavor,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.plan_bundle_addon_lock(build_plan_bundle_addon_lock_request(
                bundle,
                installation,
            ))?;
            render(json, &result, render_bundle_addon_lock_plan)?;
        }
        BundleCommands::AddonApply {
            bundle,
            install,
            flavor,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.apply_bundle_addon_lock(build_apply_bundle_addon_lock_request(
                bundle,
                installation,
                backup_output,
                replace_existing,
            ))?;
            render(json, &result, render_bundle_addon_lock_apply)?;
        }
        _ => {
            return Err(AppError::Validation(
                "internal CLI routing error: bundle addon handler received unexpected command"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
