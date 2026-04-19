use super::BundleCommands;
use super::mapping::merge_apply_mapping_overrides;
use super::output::render;
use crate::core::app::{
    ApplyBundleAppRequest, BundleApplyMappingsValue, HearthSyncApp, PlanBundleApplyRequest,
    ResolveInstallationRequest,
};
use crate::core::bundle::load_apply_mappings;
use crate::core::error::{AppError, AppResult};

pub(super) fn handle_bundle_apply_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = HearthSyncApp::new();

    match command {
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
            let installation = app.resolve_installation(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let apply_mappings = resolve_apply_mappings(
                mapping_file.as_deref(),
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            )?;
            let plan = app.plan_bundle_apply(PlanBundleApplyRequest {
                bundle_path: bundle,
                installation,
                apply_mappings,
            })?;
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
                    "Bundle: {}\nTarget: {}\nDiscovered accounts: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}",
                    item.bundle_path.display(),
                    item.target_flavor_root.display(),
                    accounts,
                    selected_accounts,
                    item.summary.paths_to_remove,
                    item.summary.files_to_add,
                    item.summary.files_to_replace,
                    item.summary.files_to_skip,
                    item.summary.files_to_preserve,
                    if item.character_mappings.is_empty() {
                        "none".to_string()
                    } else {
                        format_character_mappings(&item.character_mappings)
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
            let installation = app.resolve_installation(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let apply_mappings = resolve_apply_mappings(
                mapping_file.as_deref(),
                target_account,
                target_server,
                target_character,
                selected_accounts,
                all_accounts,
            )?;
            let result = app.apply_bundle(ApplyBundleAppRequest {
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
                    format_character_mappings(&item.character_mappings)
                };
                if item.dry_run {
                    format!(
                        "Dry run only.\nBundle: {}\nTarget: {}\nPlanned files: {}\nSelected accounts: {}\nPlanned remove: {}\nPlanned add: {}\nPlanned replace: {}\nPlanned skip: {}\nPlanned preserve: {}\nCharacter mappings: {}\nBackup: {}",
                        item.bundle_path.display(),
                        item.target_flavor_root.display(),
                        item.planned_files,
                        selected_accounts,
                        item.plan_summary.paths_to_remove,
                        item.plan_summary.files_to_add,
                        item.plan_summary.files_to_replace,
                        item.plan_summary.files_to_skip,
                        item.plan_summary.files_to_preserve,
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
        _ => {
            return Err(AppError::Validation(
                "internal CLI routing error: bundle apply handler received unexpected command"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

pub(super) fn resolve_apply_mappings(
    mapping_file: Option<&std::path::Path>,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
    selected_accounts: Vec<String>,
    all_accounts: bool,
) -> AppResult<BundleApplyMappingsValue> {
    let mut apply_mappings = if let Some(path) = mapping_file {
        load_apply_mappings(path)?.into()
    } else {
        BundleApplyMappingsValue::default()
    };
    merge_apply_mapping_overrides(
        &mut apply_mappings,
        target_account,
        target_server,
        target_character,
        selected_accounts,
        all_accounts,
    );
    Ok(apply_mappings)
}

pub(super) fn format_character_mappings(
    mappings: &[crate::core::app::CharacterMappingResult],
) -> String {
    mappings
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
