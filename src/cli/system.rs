use super::app_support::{render_with_value, stable_services};
use super::output::system::{
    render_installation_health_report, render_installation_inspection, render_installation_scan,
    render_manifest_example, render_manifest_validation,
};
use super::{InstallTargetArgs, ManifestCommands};
use crate::core::error::AppResult;

mod request;

pub(in crate::cli) use request::{ManifestExampleResult, ManifestValidationResult};

use request::{
    build_inspect_installation_request, build_manifest_example_result,
    build_manifest_validation_result,
};

pub(super) fn handle_scan(json: bool) -> AppResult<()> {
    let app = stable_services();
    render_with_value(json, || app.scan_installations(), render_installation_scan)
}

pub(super) fn handle_inspect(json: bool, install_target: InstallTargetArgs) -> AppResult<()> {
    let app = stable_services();
    render_with_value(
        json,
        || app.inspect_installation(build_inspect_installation_request(install_target)),
        render_installation_inspection,
    )
}

pub(super) fn handle_doctor(json: bool, install_target: InstallTargetArgs) -> AppResult<()> {
    let app = stable_services();
    render_with_value(
        json,
        || app.inspect_installation(build_inspect_installation_request(install_target)),
        |inspection| render_installation_health_report(&inspection.health),
    )
}

pub(super) fn handle_manifest_command(json: bool, command: ManifestCommands) -> AppResult<()> {
    match command {
        ManifestCommands::Example => {
            render_with_value(json, build_manifest_example_result, render_manifest_example)?
        }
        ManifestCommands::Validate { file } => render_with_value(
            json,
            || build_manifest_validation_result(file),
            render_manifest_validation,
        )?,
    }

    Ok(())
}
