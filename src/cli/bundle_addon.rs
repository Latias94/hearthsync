use super::BundleCommands;
use super::app_support::{extended_services, render_with_installation};
use super::output::{render_bundle_addon_lock_apply, render_bundle_addon_lock_plan};
use crate::core::error::{AppError, AppResult};

mod request;

use request::{build_apply_bundle_addon_lock_request, build_plan_bundle_addon_lock_request};

pub(super) fn handle_bundle_addon_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = extended_services();

    match command {
        BundleCommands::AddonPlan {
            bundle,
            install_target,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| build_plan_bundle_addon_lock_request(bundle, installation),
            |request| app.plan_bundle_addon_lock(request),
            render_bundle_addon_lock_plan,
        )?,
        BundleCommands::AddonApply {
            bundle,
            install_target,
            backup_output,
            replace_existing,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| {
                build_apply_bundle_addon_lock_request(
                    bundle,
                    installation,
                    backup_output,
                    replace_existing,
                )
            },
            |request| app.apply_bundle_addon_lock(request),
            render_bundle_addon_lock_apply,
        )?,
        _ => {
            return Err(AppError::Validation(
                "internal CLI routing error: bundle addon handler received unexpected command"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
