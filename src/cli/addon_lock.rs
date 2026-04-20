use super::AddonLockCommands;
use super::app_support::{extended_services, render_with_installation, render_with_value};
use super::output::{
    render_addon_lock_apply, render_addon_lock_diff, render_addon_lock_inspection,
    render_addon_lock_plan, render_addon_lock_verify, render_addon_lock_write,
};
use crate::core::error::AppResult;

mod request;

use request::{
    build_apply_addon_lock_request, build_diff_addon_lock_request,
    build_inspect_addon_lock_request, build_plan_addon_lock_request,
    build_verify_addon_lock_request, build_write_addon_lock_request,
};

pub(super) fn handle_addon_lock_command(json: bool, command: AddonLockCommands) -> AppResult<()> {
    let app = extended_services();

    match command {
        AddonLockCommands::Inspect { install_target } => render_with_installation(
            json,
            app.stable(),
            install_target,
            build_inspect_addon_lock_request,
            |request| app.inspect_addon_lock(request),
            render_addon_lock_inspection,
        )?,
        AddonLockCommands::Write { install_target } => render_with_installation(
            json,
            app.stable(),
            install_target,
            build_write_addon_lock_request,
            |request| app.write_addon_lock(request),
            render_addon_lock_write,
        )?,
        AddonLockCommands::Diff {
            left_file,
            right_file,
        } => render_with_value(
            json,
            || app.diff_addon_locks(build_diff_addon_lock_request(left_file, right_file)),
            render_addon_lock_diff,
        )?,
        AddonLockCommands::Verify {
            install_target,
            file,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| build_verify_addon_lock_request(installation, file),
            |request| app.verify_addon_lock(request),
            render_addon_lock_verify,
        )?,
        AddonLockCommands::Plan {
            install_target,
            file,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| build_plan_addon_lock_request(installation, file),
            |request| app.plan_addon_lock_sync(request),
            render_addon_lock_plan,
        )?,
        AddonLockCommands::Apply {
            install_target,
            file,
            backup_output,
            replace_existing,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| {
                build_apply_addon_lock_request(installation, file, backup_output, replace_existing)
            },
            |request| app.apply_addon_lock_sync(request),
            render_addon_lock_apply,
        )?,
    }

    Ok(())
}
