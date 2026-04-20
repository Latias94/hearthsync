use super::InstallTargetArgs;
use super::app_support::{extended_services, render_with_installation};
use super::output::{render_bundle_addon_lock_apply, render_bundle_addon_lock_plan};
use crate::core::error::AppResult;

mod request;

use request::{build_apply_bundle_addon_lock_request, build_plan_bundle_addon_lock_request};

pub(super) fn handle_bundle_addon_plan(
    json: bool,
    bundle: std::path::PathBuf,
    install_target: InstallTargetArgs,
) -> AppResult<()> {
    let app = extended_services();

    render_with_installation(
        json,
        app.stable(),
        install_target,
        |installation| build_plan_bundle_addon_lock_request(bundle, installation),
        |request| app.plan_bundle_addon_lock(request),
        render_bundle_addon_lock_plan,
    )
}

pub(super) fn handle_bundle_addon_apply(
    json: bool,
    bundle: std::path::PathBuf,
    install_target: InstallTargetArgs,
    backup_output: Option<std::path::PathBuf>,
    replace_existing: bool,
) -> AppResult<()> {
    let app = extended_services();

    render_with_installation(
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
    )
}
