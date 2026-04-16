use super::AddonCommands;
use super::AddonIndexCommands;
use super::AddonLockCommands;
use super::output::{render, render_addon_lock_plan_summary};
use crate::core::addon::index::{
    AddonIndexInstallRequest, AddonIndexUpdateRequest, inspect_addon_index,
    install_addon_from_index, update_addons_from_index,
};
use crate::core::addon::lock::{
    AddonLockApplyRequest, apply_addon_lock_sync, diff_addon_locks, inspect_addon_lock,
    plan_addon_lock_sync, verify_addon_lock, write_addon_lock,
};
use crate::core::addon::{
    InstallAddonRequest, RemoveAddonRequest, SearchAddonRequest, UpdateAddonRequest, install_addon,
    list_addons, remove_addons, search_addons, update_addons,
};
use crate::core::error::AppResult;
use crate::core::install::resolve_installation;

pub(super) fn handle_addon_command(json: bool, command: AddonCommands) -> AppResult<()> {
    match command {
        AddonCommands::Index { command } => handle_addon_index_command(json, command)?,
        AddonCommands::Lock { command } => handle_addon_lock_command(json, command)?,
        AddonCommands::Search {
            install,
            flavor,
            query,
            limit,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let results = search_addons(SearchAddonRequest {
                installation,
                query,
                limit,
            })?;
            render(json, &results, |item| {
                if item.results.is_empty() {
                    format!("Query: {}\nNo addons found.", item.query)
                } else {
                    let mut lines = vec![
                        format!("Query: {}", item.query),
                        format!("Found {} result(s):", item.results.len()),
                    ];
                    for result in &item.results {
                        lines.push(format!(
                            "- {} | provider: {} | source: {} | downloads: {} | website: {}{}",
                            result.name,
                            result.provider,
                            result.install_hint,
                            result.download_count,
                            result.website_url.as_deref().unwrap_or("none"),
                            result
                                .summary
                                .as_deref()
                                .map(|summary| format!(" | summary: {summary}"))
                                .unwrap_or_default()
                        ));
                    }
                    lines.join("\n")
                }
            })?;
        }
        AddonCommands::List { install, flavor } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let inventory = list_addons(&installation)?;
            render(json, &inventory, |item| {
                let tracked = if item.tracked_packages.is_empty() {
                    "none".to_string()
                } else {
                    item.tracked_packages
                        .iter()
                        .map(|package| {
                            format!(
                                "{} => {} [{}]",
                                package.package_id,
                                package.source.display_name(),
                                package
                                    .addons
                                    .iter()
                                    .map(|addon| addon.directory_name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let untracked = if item.untracked_addons.is_empty() {
                    "none".to_string()
                } else {
                    item.untracked_addons.join(", ")
                };
                format!(
                    "Target: {}\nRegistry: {}\nTracked packages:\n{}\nUntracked addon directories: {}",
                    item.target_addon_root.display(),
                    item.registry_path.display(),
                    tracked,
                    untracked
                )
            })?;
        }
        AddonCommands::Install {
            install,
            flavor,
            source,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = install_addon(InstallAddonRequest {
                installation,
                source,
                dry_run,
                backup_output_path: backup_output,
                replace_existing,
                metadata: None,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let replaced = if item.replaced_addons.is_empty() {
                    "none".to_string()
                } else {
                    item.replaced_addons.join(", ")
                };
                let addons = item
                    .addons
                    .iter()
                    .map(|addon| addon.directory_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                if item.dry_run {
                    format!(
                        "Dry run only.\nSource: {}\nPackage: {}\nAddons: {}\nFiles to write: {}\nWould replace: {}\nBackup: {}",
                        item.source.display_name(),
                        item.package_id,
                        addons,
                        item.files_to_write,
                        replaced,
                        backup
                    )
                } else {
                    format!(
                        "Installed package: {}\nSource: {}\nAddons: {}\nWritten files: {}\nReplaced addons: {}\nRegistry: {}\nBackup: {}",
                        item.package_id,
                        item.source.display_name(),
                        addons,
                        item.written_files,
                        replaced,
                        item.registry_path.display(),
                        backup
                    )
                }
            })?;
        }
        AddonCommands::Update {
            install,
            flavor,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = update_addons(UpdateAddonRequest {
                installation,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let packages = if item.updated_packages.is_empty() {
                    "none".to_string()
                } else {
                    item.updated_packages
                        .iter()
                        .map(|package| {
                            format!(
                                "{} [{}]",
                                package.package_id,
                                package
                                    .addons
                                    .iter()
                                    .map(|addon| addon.directory_name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                if item.dry_run {
                    format!(
                        "Dry run only.\nRegistry: {}\nPackages: {}\nFiles to write: {}\nBackup: {}",
                        item.registry_path.display(),
                        packages,
                        item.files_to_write,
                        backup
                    )
                } else {
                    format!(
                        "Updated packages: {}\nWritten files: {}\nRegistry: {}\nBackup: {}",
                        packages,
                        item.written_files,
                        item.registry_path.display(),
                        backup
                    )
                }
            })?;
        }
        AddonCommands::Remove {
            install,
            flavor,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = remove_addons(RemoveAddonRequest {
                installation,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let packages = if item.removed_packages.is_empty() {
                    "none".to_string()
                } else {
                    item.removed_packages
                        .iter()
                        .map(|package| package.package_id.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let addons = if item.removed_addons.is_empty() {
                    "none".to_string()
                } else {
                    item.removed_addons.join(", ")
                };
                if item.dry_run {
                    format!(
                        "Dry run only.\nRegistry: {}\nPackages: {}\nAddon directories: {}\nBackup: {}",
                        item.registry_path.display(),
                        packages,
                        addons,
                        backup
                    )
                } else {
                    format!(
                        "Removed packages: {}\nRemoved addon directories: {}\nRegistry: {}\nRegistry cleaned: {}\nBackup: {}",
                        packages,
                        addons,
                        item.registry_path.display(),
                        item.registry_cleaned,
                        backup
                    )
                }
            })?;
        }
    }

    Ok(())
}

fn handle_addon_index_command(json: bool, command: AddonIndexCommands) -> AppResult<()> {
    match command {
        AddonIndexCommands::Inspect { file } => {
            let inspection = inspect_addon_index(&file)?;
            render(json, &inspection, |item| {
                let packages = item
                    .index
                    .packages
                    .iter()
                    .map(|package| {
                        format!(
                            "{} {} => {}",
                            package.id,
                            package.version,
                            package.source.display_name()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "Index: {}\nName: {}\nPackages: {}\n{}",
                    item.index_path.display(),
                    item.index.name,
                    item.package_count,
                    if packages.is_empty() {
                        "none".to_string()
                    } else {
                        packages
                    }
                )
            })?;
        }
        AddonIndexCommands::Install {
            install,
            flavor,
            file,
            name,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = install_addon_from_index(AddonIndexInstallRequest {
                installation,
                index_path: file,
                name,
                dry_run,
                backup_output_path: backup_output,
                replace_existing,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .install
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let addons = item
                    .install
                    .addons
                    .iter()
                    .map(|addon| addon.directory_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                if item.install.dry_run {
                    format!(
                        "Dry run only.\nIndex: {}\nPackage: {} {}\nAddons: {}\nFiles to write: {}\nBackup: {}",
                        item.index_path.display(),
                        item.package.id,
                        item.package.version,
                        addons,
                        item.install.files_to_write,
                        backup
                    )
                } else {
                    format!(
                        "Installed index package: {} {}\nIndex: {}\nAddons: {}\nWritten files: {}\nBackup: {}",
                        item.package.id,
                        item.package.version,
                        item.index_path.display(),
                        addons,
                        item.install.written_files,
                        backup
                    )
                }
            })?;
        }
        AddonIndexCommands::Update {
            install,
            flavor,
            file,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = update_addons_from_index(AddonIndexUpdateRequest {
                installation,
                index_path: file,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .update
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let packages = item
                    .selected_packages
                    .iter()
                    .map(|package| format!("{} {}", package.id, package.version))
                    .collect::<Vec<_>>()
                    .join(", ");
                if item.update.dry_run {
                    format!(
                        "Dry run only.\nIndex: {}\nPackages: {}\nFiles to write: {}\nBackup: {}",
                        item.index_path.display(),
                        packages,
                        item.update.files_to_write,
                        backup
                    )
                } else {
                    format!(
                        "Updated index packages: {}\nIndex: {}\nWritten files: {}\nBackup: {}",
                        packages,
                        item.index_path.display(),
                        item.update.written_files,
                        backup
                    )
                }
            })?;
        }
    }

    Ok(())
}

fn handle_addon_lock_command(json: bool, command: AddonLockCommands) -> AppResult<()> {
    match command {
        AddonLockCommands::Inspect { install, flavor } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let inspection = inspect_addon_lock(&installation)?;
            render(json, &inspection, |item| {
                let packages = item
                    .lock
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
                    item.lock.generated_at,
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
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = write_addon_lock(&installation)?;
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
            let result = diff_addon_locks(&left_file, &right_file)?;
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
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = verify_addon_lock(&installation, file.as_deref())?;
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
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = plan_addon_lock_sync(&installation, file.as_deref())?;
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
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = apply_addon_lock_sync(AddonLockApplyRequest {
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
