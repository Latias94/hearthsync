use super::BundleCommands;
use super::app_support::{extended_services, resolve_cli_installation};
use super::output::{render, render_addon_lock_apply_summary, render_addon_lock_plan_summary};
use crate::core::app::{ApplyBundleAddonLockAppRequest, PlanBundleAddonLockRequest};
use crate::core::error::{AppError, AppResult};

pub(super) fn handle_bundle_addon_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = extended_services();

    match command {
        BundleCommands::AddonPlan {
            bundle,
            install,
            flavor,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.plan_bundle_addon_lock(PlanBundleAddonLockRequest {
                bundle_path: bundle,
                installation,
            })?;
            render(json, &result, |item| {
                render_addon_lock_plan_summary(
                    &format!("Bundle: {}", item.bundle_path.display()),
                    &item.plan,
                )
            })?;
        }
        BundleCommands::AddonApply {
            bundle,
            install,
            flavor,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_cli_installation(app.stable(), install, flavor)?;
            let result = app.apply_bundle_addon_lock(ApplyBundleAddonLockAppRequest {
                bundle_path: bundle,
                installation,
                backup_output_path: backup_output,
                replace_existing,
            })?;
            render(json, &result, |item| {
                render_addon_lock_apply_summary(
                    vec![
                        format!("Bundle: {}", item.bundle_path.display()),
                        format!("Embedded lock: {}", item.embedded_lock_entry),
                        format!("Installation: {}", item.apply.installation_root.display()),
                        format!(
                            "Applied: {} install, {} update, {} remove, {} metadata-only, {} unchanged",
                            item.apply.install_count,
                            item.apply.update_count,
                            item.apply.remove_count,
                            item.apply.metadata_only_count,
                            item.apply.unchanged_count
                        ),
                    ],
                    &item.apply,
                )
            })?;
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
