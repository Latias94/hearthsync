use super::AddonPolicyCommands;
use super::app_support::{
    render_with_fallible_installation, render_with_installation, stable_services,
};
use super::output::addon_policy::{render_addon_policy_inspection, render_addon_policy_mutation};
use crate::core::app::{
    AddonPolicyPinValue, AppRuntime, InspectAddonPolicyRequest, RemoveAddonPolicyAppRequest,
    SetAddonPolicyAppRequest,
};
use crate::core::error::{AppError, AppResult};

pub(super) fn handle_addon_policy_command(
    json: bool,
    runtime: AppRuntime,
    command: AddonPolicyCommands,
) -> AppResult<()> {
    let app = stable_services(runtime);

    match command {
        AddonPolicyCommands::Inspect { install_target } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| InspectAddonPolicyRequest { installation },
            |request| app.inspect_addon_policy(request),
            render_addon_policy_inspection,
        )?,
        AddonPolicyCommands::Set {
            install_target,
            package,
            ignored,
            pinned_version,
            pinned_file_id,
            release_channel,
            allow_prerelease,
            install_dependencies,
        } => render_with_fallible_installation(
            json,
            &app,
            install_target,
            |installation| {
                Ok(SetAddonPolicyAppRequest {
                    installation,
                    package,
                    ignored,
                    pin: build_addon_policy_pin(pinned_version, pinned_file_id)?,
                    release_channel: release_channel.map(Into::into),
                    allow_prerelease,
                    install_dependencies,
                })
            },
            |request| app.set_addon_policy(request),
            render_addon_policy_mutation,
        )?,
        AddonPolicyCommands::Remove {
            install_target,
            package,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| RemoveAddonPolicyAppRequest {
                installation,
                package,
            },
            |request| app.remove_addon_policy(request),
            render_addon_policy_mutation,
        )?,
    }

    Ok(())
}

fn build_addon_policy_pin(
    pinned_version: Option<String>,
    pinned_file_id: Option<u32>,
) -> AppResult<Option<AddonPolicyPinValue>> {
    match (pinned_version, pinned_file_id) {
        (Some(_version), Some(_)) => Err(AppError::Validation(
            "addon policy cannot pin both a version and a file id".to_string(),
        )),
        (Some(version), None) => {
            if version.trim().is_empty() {
                return Err(AppError::Validation(
                    "addon policy pinned version cannot be empty".to_string(),
                ));
            }
            Ok(Some(AddonPolicyPinValue::Version {
                value: version.trim().to_string(),
            }))
        }
        (None, Some(file_id)) => Ok(Some(AddonPolicyPinValue::FileId { value: file_id })),
        (None, None) => Ok(None),
    }
}
