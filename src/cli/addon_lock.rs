use super::AddonLockCommands;
use super::output::{render, render_addon_lock_plan_summary};
use crate::core::addon::lock::AddonLockApplyRequest;
use crate::core::app::{
    DiffAddonLockRequest, HearthSyncApp, InspectAddonLockRequest, PlanAddonLockSyncRequest,
    ResolveInstallationRequest, VerifyAddonLockRequest, WriteAddonLockRequest,
};
use crate::core::error::AppResult;

pub(super) fn handle_addon_lock_command(json: bool, command: AddonLockCommands) -> AppResult<()> {
    let app = HearthSyncApp::new();
    let service = app.addon_locks();
    let installation_service = app.installations();

    match command {
        AddonLockCommands::Inspect { install, flavor } => {
            let installation = installation_service.resolve(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let inspection = service.inspect(InspectAddonLockRequest { installation })?;
            render(json, &inspection, |item| {
                let packages = item
                    .packages
                    .iter()
                    .map(|package| {
                        format!(
                            "{} {} => {} ({})",
                            package.package_id,
                            package.version.as_deref().unwrap_or("unknown"),
                            package.addon_directories.join(", "),
                            package.content_sha256
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "Lock: {}\nGenerated: {}\nPackages: {}\n{}",
                    item.lock_path.display(),
                    item.generated_at,
                    item.package_count,
                    if packages.is_empty() {
                        "none".to_string()
                    } else {
                        packages
                    }
                )
            })?;
        }
        AddonLockCommands::Write { install, flavor } => {
            let installation = installation_service.resolve(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let result = service.write(WriteAddonLockRequest { installation })?;
            render(json, &result, |item| {
                if item.removed {
                    format!(
                        "Removed addon lock: {}\nTracked packages: 0",
                        item.lock_path.display()
                    )
                } else {
                    format!(
                        "Wrote addon lock: {}\nTracked packages: {}",
                        item.lock_path.display(),
                        item.package_count
                    )
                }
            })?;
        }
        AddonLockCommands::Diff {
            left_file,
            right_file,
        } => {
            let result = service.diff(DiffAddonLockRequest {
                left_lock_path: left_file,
                right_lock_path: right_file,
            })?;
            render(json, &result, |item| {
                let mut lines = vec![
                    format!("Left: {}", item.left_label),
                    format!("Right: {}", item.right_label),
                    format!(
                        "Summary: {} changed, {} added, {} removed, {} unchanged",
                        item.changed_packages.len(),
                        item.added_packages.len(),
                        item.removed_packages.len(),
                        item.unchanged_packages
                    ),
                ];

                if item.identical {
                    lines.push("Result: identical".to_string());
                    return lines.join("\n");
                }

                if !item.changed_packages.is_empty() {
                    lines.push("Changed packages:".to_string());
                    for package in &item.changed_packages {
                        let changed_fields = package
                            .changes
                            .iter()
                            .map(|change| change.field.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        lines.push(format!(
                            "- {} ({})",
                            package
                                .left
                                .name
                                .as_deref()
                                .unwrap_or(&package.left.package_id),
                            changed_fields
                        ));
                    }
                }

                if !item.added_packages.is_empty() {
                    lines.push("Added packages:".to_string());
                    for package in &item.added_packages {
                        lines.push(format!(
                            "- {}",
                            package.name.as_deref().unwrap_or(&package.package_id)
                        ));
                    }
                }

                if !item.removed_packages.is_empty() {
                    lines.push("Removed packages:".to_string());
                    for package in &item.removed_packages {
                        lines.push(format!(
                            "- {}",
                            package.name.as_deref().unwrap_or(&package.package_id)
                        ));
                    }
                }

                lines.join("\n")
            })?;
        }
        AddonLockCommands::Verify {
            install,
            flavor,
            file,
        } => {
            let installation = installation_service.resolve(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let result = service.verify(VerifyAddonLockRequest {
                installation,
                lock_path: file,
            })?;
            render(json, &result, |item| {
                let mut lines = vec![
                    format!("Lock: {}", item.lock_path.display()),
                    format!("Installation: {}", item.installation_root.display()),
                    format!(
                        "Summary: {} changed, {} added, {} removed, {} unchanged",
                        item.diff.changed_packages.len(),
                        item.diff.added_packages.len(),
                        item.diff.removed_packages.len(),
                        item.diff.unchanged_packages
                    ),
                ];

                if item.matches {
                    lines.push("Result: verified".to_string());
                    return lines.join("\n");
                }

                lines.push("Result: drift detected".to_string());

                if !item.missing_addon_directories.is_empty() {
                    lines.push("Missing tracked addon directories:".to_string());
                    for issue in &item.missing_addon_directories {
                        lines.push(format!(
                            "- {} => {}",
                            issue.package_id,
                            issue.missing_addon_directories.join(", ")
                        ));
                    }
                }

                if !item.untracked_addons.is_empty() {
                    lines.push(format!(
                        "Untracked addon directories: {}",
                        item.untracked_addons.join(", ")
                    ));
                }

                if !item.diff.changed_packages.is_empty() {
                    lines.push("Changed packages:".to_string());
                    for package in &item.diff.changed_packages {
                        let changed_fields = package
                            .changes
                            .iter()
                            .map(|change| change.field.as_str())
                            .collect::<Vec<_>>()
                            .join(", ");
                        lines.push(format!(
                            "- {} ({})",
                            package
                                .left
                                .name
                                .as_deref()
                                .unwrap_or(&package.left.package_id),
                            changed_fields
                        ));
                    }
                }

                if !item.diff.added_packages.is_empty() {
                    lines.push("Unexpected tracked packages:".to_string());
                    for package in &item.diff.added_packages {
                        lines.push(format!(
                            "- {}",
                            package.name.as_deref().unwrap_or(&package.package_id)
                        ));
                    }
                }

                if !item.diff.removed_packages.is_empty() {
                    lines.push("Missing expected packages:".to_string());
                    for package in &item.diff.removed_packages {
                        lines.push(format!(
                            "- {}",
                            package.name.as_deref().unwrap_or(&package.package_id)
                        ));
                    }
                }

                lines.join("\n")
            })?;
        }
        AddonLockCommands::Plan {
            install,
            flavor,
            file,
        } => {
            let installation = installation_service.resolve(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let result = service.plan_sync(PlanAddonLockSyncRequest {
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
            let installation = installation_service.resolve(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let result = service.apply_sync(AddonLockApplyRequest {
                installation,
                lock_path: file,
                backup_output_path: backup_output,
                replace_existing,
                source_overrides: Vec::new(),
            })?;
            render(json, &result, |item| {
                let mut lines = vec![
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
                ];

                if !item.untracked_addons.is_empty() {
                    lines.push(format!(
                        "Untracked addon directories remain: {}",
                        item.untracked_addons.join(", ")
                    ));
                }
                lines.push(if item.verification.matches {
                    "Verification: matches".to_string()
                } else {
                    format!(
                        "Verification: drift remains ({} changed, {} added, {} removed)",
                        item.verification.diff.changed_packages.len(),
                        item.verification.diff.added_packages.len(),
                        item.verification.diff.removed_packages.len()
                    )
                });
                lines.join("\n")
            })?;
        }
    }

    Ok(())
}
