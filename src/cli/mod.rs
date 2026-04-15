mod args;

use clap::Parser;
use serde::Serialize;

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
use crate::core::backup::{
    BackupGroup, BackupRequest, RestoreBackupRequest, create_backup, list_backups,
    restore_backup_selection,
};
use crate::core::bundle::{
    BundleAddonLockApplyRequest, BundleApplyMappings, PackBundleRequest, UnpackBundleRequest,
    apply_bundle_addon_lock, inspect_bundle, load_apply_mappings, pack_bundle,
    plan_bundle_addon_lock, plan_bundle_apply, unpack_bundle,
};
use crate::core::error::AppResult;
use crate::core::install::{inspect_installation, resolve_installation, scan_installations};
use crate::core::manifest::{example_manifest, load_manifest};

pub use args::*;

pub fn run() -> AppResult<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan => {
            let installations = scan_installations()?;
            render(cli.json, &installations, |items| {
                if items.is_empty() {
                    "No World of Warcraft installations detected.".to_string()
                } else {
                    let mut lines = vec![format!("Detected {} installation(s):", items.len())];
                    for item in items {
                        lines.push(format!(
                            "- {} => {}",
                            item.flavor.as_str(),
                            item.flavor_root.display()
                        ));
                    }
                    lines.join("\n")
                }
            })?;
        }
        Commands::Inspect { install, flavor } => {
            let inspection = inspect_installation(&install, flavor.map(Into::into))?;
            render(cli.json, &inspection, |item| {
                format!(
                    "Flavor: {}\nProduct root: {}\nFlavor root: {}\nAddOns: {}\nWTF: {}\nFonts: {}\nHealth: {}",
                    item.installation.flavor.as_str(),
                    item.product_root.display(),
                    item.installation.flavor_root.display(),
                    item.installation.addon_dir.display(),
                    item.installation.wtf_dir.display(),
                    item.installation.fonts_dir.display(),
                    item.health.summary()
                )
            })?;
        }
        Commands::Doctor { install, flavor } => {
            let inspection = inspect_installation(&install, flavor.map(Into::into))?;
            render(cli.json, &inspection.health, |health| health.to_report())?;
        }
        Commands::Backup { command } => match command {
            BackupCommands::Create {
                install,
                flavor,
                output,
            } => {
                let installation = resolve_installation(&install, flavor.map(Into::into))?;
                let backup = create_backup(BackupRequest {
                    installation,
                    output_path: output,
                    groups: vec![
                        BackupGroup::Addons,
                        BackupGroup::Wtf,
                        BackupGroup::Fonts,
                        BackupGroup::InterfaceAssets,
                    ],
                    label: None,
                })?;
                render(cli.json, &backup, |item| {
                    format!(
                        "Created backup: {}\nArchived files: {}\nGroups: {}",
                        item.archive_path.display(),
                        item.archived_files,
                        item.metadata
                            .groups
                            .iter()
                            .map(BackupGroup::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            }
            BackupCommands::List { dir } => {
                let backups = list_backups(dir.as_deref())?;
                render(cli.json, &backups, |item| {
                    if item.entries.is_empty() {
                        format!(
                            "Backup dir: {}\nNo backups found.",
                            item.backup_dir.display()
                        )
                    } else {
                        let mut lines = vec![
                            format!("Backup dir: {}", item.backup_dir.display()),
                            format!("Found {} backup(s):", item.entries.len()),
                        ];
                        for entry in &item.entries {
                            let groups = entry
                                .metadata
                                .groups
                                .iter()
                                .map(BackupGroup::as_str)
                                .collect::<Vec<_>>()
                                .join(", ");
                            let label = entry.metadata.label.as_deref().unwrap_or("none");
                            lines.push(format!(
                                "- {} | label: {} | created: {} | flavor: {} | groups: {} | size: {} bytes | path: {}",
                                entry.backup_id,
                                label,
                                entry.metadata.created_at,
                                entry.metadata.flavor,
                                groups,
                                entry.archive_size_bytes,
                                entry.archive_path.display()
                            ));
                        }
                        lines.join("\n")
                    }
                })?;
            }
            BackupCommands::Restore {
                install,
                flavor,
                archive,
                id,
                dir,
            } => {
                let installation = resolve_installation(&install, flavor.map(Into::into))?;
                let restored = restore_backup_selection(RestoreBackupRequest {
                    installation,
                    archive_path: archive,
                    backup_id: id,
                    backup_dir: dir,
                })?;
                render(cli.json, &restored, |item| {
                    format!(
                        "Restored backup: {}\nRestored files: {}\nCreated at: {}\nLabel: {}\nGroups: {}",
                        item.archive_path.display(),
                        item.restored_files,
                        item.metadata.created_at,
                        item.metadata.label.as_deref().unwrap_or("none"),
                        item.metadata
                            .groups
                            .iter()
                            .map(BackupGroup::as_str)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;
            }
        },
        Commands::Bundle { command } => match command {
            BundleCommands::Pack {
                install,
                flavor,
                manifest,
                output,
            } => {
                let installation = resolve_installation(&install, flavor.map(Into::into))?;
                let manifest_base_dir = manifest.parent().map(|path| path.to_path_buf());
                let manifest = load_manifest(&manifest)?;
                let bundle = pack_bundle(PackBundleRequest {
                    installation,
                    manifest,
                    output_path: output,
                    manifest_base_dir,
                })?;
                render(cli.json, &bundle, |item| {
                    format!(
                        "Created bundle: {}\nArchived files: {}\nPackage: {}",
                        item.archive_path.display(),
                        item.archived_files,
                        item.manifest.package.name
                    )
                })?;
            }
            BundleCommands::Inspect { bundle } => {
                let inspection = inspect_bundle(&bundle)?;
                render(cli.json, &inspection, |item| {
                    let characters = item
                        .manifest
                        .resources
                        .wtf_characters
                        .iter()
                        .map(|character| {
                            format!(
                                "{}/{}/{}",
                                character
                                    .source_account
                                    .as_deref()
                                    .unwrap_or("<unknown-account>"),
                                character.source_server,
                                character.source_character
                            )
                        })
                        .collect::<Vec<_>>();
                    format!(
                        "Bundle: {}\nPackage: {}\nSource flavor: {}\nFiles: {}\nAddOns: {}\nWTF common: {}\nWTF characters: {}\nFonts: {}\nInterface assets: {}\nCharacters: {}",
                        item.archive_path.display(),
                        item.manifest.package.name,
                        item.manifest.source.flavor.as_str(),
                        item.entries.total_files,
                        item.entries.addons,
                        item.entries.wtf_common,
                        item.entries.wtf_characters,
                        item.entries.fonts,
                        item.entries.interface_assets,
                        if characters.is_empty() {
                            "none".to_string()
                        } else {
                            characters.join(", ")
                        }
                    )
                })?;
            }
            BundleCommands::Plan {
                bundle,
                install,
                flavor,
                mapping_file,
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            } => {
                let installation = resolve_installation(&install, flavor.map(Into::into))?;
                let mut apply_mappings = if let Some(path) = mapping_file {
                    load_apply_mappings(&path)?
                } else {
                    BundleApplyMappings::default()
                };
                merge_apply_mapping_overrides(
                    &mut apply_mappings,
                    target_account,
                    target_server,
                    target_character,
                    selected_accounts,
                    all_accounts,
                );
                let plan = plan_bundle_apply(&bundle, &installation, &apply_mappings)?;
                render(cli.json, &plan, |item| {
                    let accounts = if item.discovered_accounts.is_empty() {
                        "none".to_string()
                    } else {
                        item.discovered_accounts
                            .iter()
                            .map(|account| {
                                format!(
                                    "{}({} chars)",
                                    account.account_name,
                                    account.characters.len()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    let selected_accounts = if item.selected_target_accounts.is_empty() {
                        "none".to_string()
                    } else {
                        item.selected_target_accounts.join(", ")
                    };
                    format!(
                        "Bundle: {}\nTarget: {}\nDiscovered accounts: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nPlanned rewrite: {}\nCharacter mappings: {}",
                        item.bundle_path.display(),
                        item.target_flavor_root.display(),
                        accounts,
                        selected_accounts,
                        item.summary.paths_to_remove,
                        item.summary.files_to_add,
                        item.summary.files_to_replace,
                        item.summary.files_to_skip,
                        item.summary.files_to_preserve,
                        item.summary.files_to_rewrite,
                        if item.character_mappings.is_empty() {
                            "none".to_string()
                        } else {
                            item.character_mappings
                                .iter()
                                .map(|mapping| {
                                    format!(
                                        "{}/{}/{} -> {}/{}/{}",
                                        mapping
                                            .source_account
                                            .as_deref()
                                            .unwrap_or("<unknown-account>"),
                                        mapping.source_server,
                                        mapping.source_character,
                                        mapping.target_account,
                                        mapping.target_server,
                                        mapping.target_character
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(", ")
                        }
                    )
                })?;
            }
            BundleCommands::Unpack {
                bundle,
                install,
                flavor,
                dry_run,
                backup_output,
                mapping_file,
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            } => {
                let installation = resolve_installation(&install, flavor.map(Into::into))?;
                let mut apply_mappings = if let Some(path) = mapping_file {
                    load_apply_mappings(&path)?
                } else {
                    BundleApplyMappings::default()
                };
                merge_apply_mapping_overrides(
                    &mut apply_mappings,
                    target_account,
                    target_server,
                    target_character,
                    selected_accounts,
                    all_accounts,
                );
                let result = unpack_bundle(UnpackBundleRequest {
                    bundle_path: bundle,
                    installation,
                    dry_run,
                    backup_output_path: backup_output,
                    apply_mappings,
                })?;
                render(cli.json, &result, |item| {
                    let backup = item
                        .backup_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "none".to_string());
                    let selected_accounts = if item.selected_target_accounts.is_empty() {
                        "none".to_string()
                    } else {
                        item.selected_target_accounts.join(", ")
                    };
                    let mapping_summary = if item.character_mappings.is_empty() {
                        "none".to_string()
                    } else {
                        item.character_mappings
                            .iter()
                            .map(|mapping| {
                                format!(
                                    "{}/{}/{} -> {}/{}/{}",
                                    mapping
                                        .source_account
                                        .as_deref()
                                        .unwrap_or("<unknown-account>"),
                                    mapping.source_server,
                                    mapping.source_character,
                                    mapping.target_account,
                                    mapping.target_server,
                                    mapping.target_character
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    if item.dry_run {
                        format!(
                            "Dry run only.\nBundle: {}\nTarget: {}\nPlanned files: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nPlanned rewrite: {}\nCharacter mappings: {}\nBackup: {}",
                            item.bundle_path.display(),
                            item.target_flavor_root.display(),
                            item.planned_files,
                            selected_accounts,
                            item.plan_summary.paths_to_remove,
                            item.plan_summary.files_to_add,
                            item.plan_summary.files_to_replace,
                            item.plan_summary.files_to_skip,
                            item.plan_summary.files_to_preserve,
                            item.plan_summary.files_to_rewrite,
                            mapping_summary,
                            backup
                        )
                    } else {
                        format!(
                            "Unpacked bundle: {}\nTarget: {}\nWritten files: {}\nRewritten files: {}\nSelected accounts: {}\nCharacter mappings: {}\nBackup: {}",
                            item.bundle_path.display(),
                            item.target_flavor_root.display(),
                            item.written_files,
                            item.rewritten_files,
                            selected_accounts,
                            mapping_summary,
                            backup
                        )
                    }
                })?;
            }
            BundleCommands::AddonPlan {
                bundle,
                install,
                flavor,
            } => {
                let installation = resolve_installation(&install, flavor.map(Into::into))?;
                let result = plan_bundle_addon_lock(&bundle, &installation)?;
                render(cli.json, &result, |item| {
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
                render(cli.json, &result, |item| {
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
        },
        Commands::Addon { command } => match command {
            AddonCommands::Index { command } => match command {
                AddonIndexCommands::Inspect { file } => {
                    let inspection = inspect_addon_index(&file)?;
                    render(cli.json, &inspection, |item| {
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
                    render(cli.json, &result, |item| {
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
                    render(cli.json, &result, |item| {
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
            },
            AddonCommands::Lock { command } => match command {
                AddonLockCommands::Inspect { install, flavor } => {
                    let installation = resolve_installation(&install, flavor.map(Into::into))?;
                    let inspection = inspect_addon_lock(&installation)?;
                    render(cli.json, &inspection, |item| {
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
                    render(cli.json, &result, |item| {
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
                    render(cli.json, &result, |item| {
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
                    render(cli.json, &result, |item| {
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
                    render(cli.json, &result, |item| {
                        let mut lines = vec![
                            format!("Lock: {}", item.lock_path.display()),
                            format!("Installation: {}", item.installation_root.display()),
                            format!(
                                "Summary: {} install, {} update, {} remove, {} metadata-only, {} unchanged, {} blocked",
                                item.install_count,
                                item.update_count,
                                item.remove_count,
                                item.metadata_only_count,
                                item.unchanged_count,
                                item.blocked_count
                            ),
                        ];

                        if !item.untracked_addons.is_empty() {
                            lines.push(format!(
                                "Untracked addon directories: {}",
                                item.untracked_addons.join(", ")
                            ));
                        }

                        if item.actions.is_empty() {
                            lines.push("No sync actions required.".to_string());
                            return lines.join("\n");
                        }

                        lines.push("Actions:".to_string());
                        for action in &item.actions {
                            let reason = if action.reasons.is_empty() {
                                "no details".to_string()
                            } else {
                                action.reasons.join("; ")
                            };
                            let mut suffix = String::new();
                            if action.requires_replace_existing {
                                suffix.push_str(" | requires --replace-existing");
                            }
                            if !action.blocked_reasons.is_empty() {
                                suffix.push_str(&format!(
                                    " | blocked: {}",
                                    action.blocked_reasons.join("; ")
                                ));
                            }
                            lines.push(format!(
                                "- {:?}: {} ({}){}",
                                action.kind, action.package_id, reason, suffix
                            ));
                        }

                        lines.join("\n")
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
                    render(cli.json, &result, |item| {
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
            },
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
                render(cli.json, &results, |item| {
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
                render(cli.json, &inventory, |item| {
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
                render(cli.json, &result, |item| {
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
                render(cli.json, &result, |item| {
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
                render(cli.json, &result, |item| {
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
        },
        Commands::Manifest { command } => match command {
            ManifestCommands::Example => {
                print!("{}", example_manifest()?);
            }
            ManifestCommands::Validate { file } => {
                let manifest = load_manifest(&file)?;
                manifest.validate()?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "status": "ok",
                            "path": file,
                        }))?
                    );
                } else {
                    println!("Manifest is valid: {}", file.display());
                }
            }
        },
    }

    Ok(())
}

fn merge_apply_mapping_overrides(
    apply_mappings: &mut BundleApplyMappings,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
    selected_accounts: Vec<String>,
    all_accounts: bool,
) {
    if target_account.is_some() {
        apply_mappings.target_account = target_account;
    }
    if target_server.is_some() {
        apply_mappings.target_server = target_server;
    }
    if target_character.is_some() {
        apply_mappings.target_character = target_character;
    }
    if !selected_accounts.is_empty() {
        apply_mappings.selected_accounts = selected_accounts;
    }
    if all_accounts {
        apply_mappings.all_accounts = true;
    }
}

fn render_addon_lock_plan_summary(
    header: &str,
    item: &crate::core::addon::lock::AddonLockPlanResult,
) -> String {
    let mut lines = vec![
        header.to_string(),
        format!("Embedded/lock path: {}", item.lock_path.display()),
        format!("Installation: {}", item.installation_root.display()),
        format!(
            "Summary: {} install, {} update, {} remove, {} metadata-only, {} unchanged, {} blocked",
            item.install_count,
            item.update_count,
            item.remove_count,
            item.metadata_only_count,
            item.unchanged_count,
            item.blocked_count
        ),
    ];

    if !item.untracked_addons.is_empty() {
        lines.push(format!(
            "Untracked addon directories: {}",
            item.untracked_addons.join(", ")
        ));
    }

    if item.actions.is_empty() {
        lines.push("No sync actions required.".to_string());
        return lines.join("\n");
    }

    lines.push("Actions:".to_string());
    for action in &item.actions {
        let reason = if action.reasons.is_empty() {
            "no details".to_string()
        } else {
            action.reasons.join("; ")
        };
        let mut suffix = String::new();
        if action.requires_replace_existing {
            suffix.push_str(" | requires --replace-existing");
        }
        if !action.blocked_reasons.is_empty() {
            suffix.push_str(&format!(
                " | blocked: {}",
                action.blocked_reasons.join("; ")
            ));
        }
        lines.push(format!(
            "- {:?}: {} ({}){}",
            action.kind, action.package_id, reason, suffix
        ));
    }

    lines.join("\n")
}

fn render<T, F>(json: bool, value: &T, text_renderer: F) -> AppResult<()>
where
    T: Serialize,
    F: FnOnce(&T) -> String,
{
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", text_renderer(value));
    }

    Ok(())
}
