use super::BundleCommands;
use super::mapping::merge_apply_mapping_overrides;
use super::output::{render, render_addon_lock_plan_summary};
use crate::core::bundle::{
    BundleAddonLockApplyRequest, BundleApplyMappings, PackBundleRequest, UnpackBundleRequest,
    apply_bundle_addon_lock, inspect_bundle, load_apply_mappings, pack_bundle,
    plan_bundle_addon_lock, plan_bundle_apply, unpack_bundle,
};
use crate::core::error::AppResult;
use crate::core::install::resolve_installation;
use crate::core::manifest::load_manifest;

pub(super) fn handle_bundle_command(json: bool, command: BundleCommands) -> AppResult<()> {
    match command {
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
            render(json, &bundle, |item| {
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
            render(json, &inspection, |item| {
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
            render(json, &plan, |item| {
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
            render(json, &result, |item| {
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
    }

    Ok(())
}
