use super::BundleCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::mapping::merge_apply_mapping_overrides;
use super::output::{render, render_bundle_apply, render_bundle_apply_plan};
use crate::core::app::{ApplyBundleAppRequest, BundleApplyMappingsValue, PlanBundleApplyRequest};
use crate::core::bundle::load_apply_mappings;
use crate::core::error::{AppError, AppResult};

pub(super) fn handle_bundle_apply_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = stable_services();

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
            let installation = resolve_cli_installation(&app, install, flavor)?;
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
            render(json, &plan, render_bundle_apply_plan)?;
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
            let installation = resolve_cli_installation(&app, install, flavor)?;
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
            render(json, &result, render_bundle_apply)?;
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
        BundleApplyMappingsValue::from_domain(load_apply_mappings(path)?)
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
