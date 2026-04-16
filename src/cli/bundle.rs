use super::BundleCommands;
use super::bundle_addon::handle_bundle_addon_command;
use super::bundle_apply::handle_bundle_apply_command;
use super::bundle_archive::handle_bundle_archive_command;
use crate::core::error::AppResult;

pub(super) fn handle_bundle_command(json: bool, command: BundleCommands) -> AppResult<()> {
    match command {
        command @ (BundleCommands::Pack { .. } | BundleCommands::Inspect { .. }) => {
            handle_bundle_archive_command(json, command)?
        }
        command @ (BundleCommands::Plan { .. } | BundleCommands::Unpack { .. }) => {
            handle_bundle_apply_command(json, command)?
        }
        command @ (BundleCommands::AddonPlan { .. } | BundleCommands::AddonApply { .. }) => {
            handle_bundle_addon_command(json, command)?
        }
    }

    Ok(())
}
