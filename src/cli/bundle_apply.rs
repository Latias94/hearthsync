use super::BundleCommands;
use super::app_support::{render_with_apply_target, stable_services};
use super::output::{render_bundle_apply, render_bundle_apply_plan};
use crate::core::error::{AppError, AppResult};

mod request;

use request::{build_apply_bundle_request, build_plan_bundle_apply_request};

pub(super) fn handle_bundle_apply_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        BundleCommands::Plan {
            bundle,
            install_target,
            apply_mapping,
        } => render_with_apply_target(
            json,
            &app,
            install_target,
            apply_mapping,
            |target| {
                build_plan_bundle_apply_request(bundle, target.installation, target.apply_mappings)
            },
            |request| app.plan_bundle_apply(request),
            render_bundle_apply_plan,
        )?,
        BundleCommands::Unpack {
            bundle,
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
                build_apply_bundle_request(
                    bundle,
                    target.installation,
                    dry_run,
                    backup_output,
                    target.apply_mappings,
                )
            },
            |request| app.apply_bundle(request),
            render_bundle_apply,
        )?,
        _ => {
            return Err(AppError::Validation(
                "internal CLI routing error: bundle apply handler received unexpected command"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
