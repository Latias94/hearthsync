use super::BundleCommands;
use super::bundle_addon::{handle_bundle_addon_apply, handle_bundle_addon_plan};
use super::bundle_apply::{handle_bundle_plan, handle_bundle_unpack};
use super::bundle_archive::{handle_bundle_inspect, handle_bundle_pack};
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

pub(super) fn handle_bundle_command(
    json: bool,
    runtime: AppRuntime,
    command: BundleCommands,
) -> AppResult<()> {
    match command {
        BundleCommands::Pack {
            install_target,
            manifest,
            output,
        } => handle_bundle_pack(json, runtime, install_target, manifest, output)?,
        BundleCommands::Inspect { bundle } => handle_bundle_inspect(json, runtime, bundle)?,
        BundleCommands::Plan {
            bundle,
            install_target,
            apply_mapping,
        } => handle_bundle_plan(json, runtime, bundle, install_target, apply_mapping)?,
        BundleCommands::Unpack {
            bundle,
            install_target,
            dry_run,
            backup_output,
            apply_mapping,
        } => handle_bundle_unpack(
            json,
            runtime,
            bundle,
            install_target,
            dry_run,
            backup_output,
            apply_mapping,
        )?,
        BundleCommands::AddonPlan {
            bundle,
            install_target,
        } => handle_bundle_addon_plan(json, runtime, bundle, install_target)?,
        BundleCommands::AddonApply {
            bundle,
            install_target,
            backup_output,
            replace_existing,
        } => handle_bundle_addon_apply(
            json,
            runtime,
            bundle,
            install_target,
            backup_output,
            replace_existing,
        )?,
    }

    Ok(())
}
