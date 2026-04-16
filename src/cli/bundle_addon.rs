use super::BundleCommands;
use super::output::{render, render_addon_lock_plan_summary};
use crate::core::bundle::{
    BundleAddonLockApplyRequest, apply_bundle_addon_lock, plan_bundle_addon_lock,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::resolve_installation;

pub(super) fn handle_bundle_addon_command(json: bool, command: BundleCommands) -> AppResult<()> {
    match command {
        BundleCommands::AddonPlan {
            bundle,
            install,
            flavor,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = plan_bundle_addon_lock(&bundle, &installation)?;
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
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = apply_bundle_addon_lock(BundleAddonLockApplyRequest {
                bundle_path: bundle,
                installation,
                backup_output_path: backup_output,
                replace_existing,
            })?;
            render(json, &result, |item| {
                let mut lines = vec![
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
                ];
                if !item.apply.untracked_addons.is_empty() {
                    lines.push(format!(
                        "Untracked addon directories remain: {}",
                        item.apply.untracked_addons.join(", ")
                    ));
                }
                lines.push(if item.apply.verification.matches {
                    "Verification: matches".to_string()
                } else {
                    format!(
                        "Verification: drift remains ({} changed, {} added, {} removed)",
                        item.apply.verification.diff.changed_packages.len(),
                        item.apply.verification.diff.added_packages.len(),
                        item.apply.verification.diff.removed_packages.len()
                    )
                });
                lines.join("\n")
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
