use super::AddonLockCommands;
use super::app_support::{extended_services, resolve_cli_installation};
use super::output::{
    render, render_addon_lock_apply, render_addon_lock_diff, render_addon_lock_inspection,
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
        AddonLockCommands::Inspect { install, flavor } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let inspection =
                app.inspect_addon_lock(build_inspect_addon_lock_request(installation))?;
            render(json, &inspection, render_addon_lock_inspection)?;
        }
        AddonLockCommands::Write { install, flavor } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.write_addon_lock(build_write_addon_lock_request(installation))?;
            render(json, &result, render_addon_lock_write)?;
        }
        AddonLockCommands::Diff {
            left_file,
            right_file,
        } => {
            let result =
                app.diff_addon_locks(build_diff_addon_lock_request(left_file, right_file))?;
            render(json, &result, render_addon_lock_diff)?;
        }
        AddonLockCommands::Verify {
            install,
            flavor,
            file,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result =
                app.verify_addon_lock(build_verify_addon_lock_request(installation, file))?;
            render(json, &result, render_addon_lock_verify)?;
        }
        AddonLockCommands::Plan {
            install,
            flavor,
            file,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result =
                app.plan_addon_lock_sync(build_plan_addon_lock_request(installation, file))?;
            render(json, &result, render_addon_lock_plan)?;
        }
        AddonLockCommands::Apply {
            install,
            flavor,
            file,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.apply_addon_lock_sync(build_apply_addon_lock_request(
                installation,
                file,
                backup_output,
                replace_existing,
            ))?;
            render(json, &result, render_addon_lock_apply)?;
        }
    }

    Ok(())
}
