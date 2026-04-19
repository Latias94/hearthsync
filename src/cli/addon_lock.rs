use super::AddonLockCommands;
use super::app_support::{extended_services, resolve_cli_installation};
use super::output::{
    render, render_addon_lock_apply_summary, render_addon_lock_diff, render_addon_lock_inspection,
    render_addon_lock_plan_summary, render_addon_lock_verify, render_addon_lock_write,
};
use crate::core::app::{
    ApplyAddonLockAppRequest, DiffAddonLockRequest, InspectAddonLockRequest,
    PlanAddonLockSyncRequest, VerifyAddonLockRequest, WriteAddonLockRequest,
};
use crate::core::error::AppResult;

pub(super) fn handle_addon_lock_command(json: bool, command: AddonLockCommands) -> AppResult<()> {
    let app = extended_services();

    match command {
        AddonLockCommands::Inspect { install, flavor } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let inspection = app.inspect_addon_lock(InspectAddonLockRequest { installation })?;
            render(json, &inspection, render_addon_lock_inspection)?;
        }
        AddonLockCommands::Write { install, flavor } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.write_addon_lock(WriteAddonLockRequest { installation })?;
            render(json, &result, render_addon_lock_write)?;
        }
        AddonLockCommands::Diff {
            left_file,
            right_file,
        } => {
            let result = app.diff_addon_locks(DiffAddonLockRequest {
                left_lock_path: left_file,
                right_lock_path: right_file,
            })?;
            render(json, &result, render_addon_lock_diff)?;
        }
        AddonLockCommands::Verify {
            install,
            flavor,
            file,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.verify_addon_lock(VerifyAddonLockRequest {
                installation,
                lock_path: file,
            })?;
            render(json, &result, render_addon_lock_verify)?;
        }
        AddonLockCommands::Plan {
            install,
            flavor,
            file,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.plan_addon_lock_sync(PlanAddonLockSyncRequest {
                installation,
                lock_path: file,
            })?;
            render(json, &result, |item| {
                render_addon_lock_plan_summary(&format!("Lock: {}", item.lock_path.display()), item)
            })?;
        }
        AddonLockCommands::Apply {
            install,
            flavor,
            file,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.apply_addon_lock_sync(ApplyAddonLockAppRequest {
                installation,
                lock_path: file,
                backup_output_path: backup_output,
                replace_existing,
                source_overrides: Vec::new(),
            })?;
            render(json, &result, |item| {
                render_addon_lock_apply_summary(
                    vec![
                        format!("Lock: {}", item.lock_path.display()),
                        format!("Installation: {}", item.installation_root.display()),
                        format!(
                            "Applied: {} install, {} update, {} remove, {} metadata-only, {} unchanged",
                            item.install_count,
                            item.update_count,
                            item.remove_count,
                            item.metadata_only_count,
                            item.unchanged_count
                        ),
                    ],
                    item,
                )
            })?;
        }
    }

    Ok(())
}
