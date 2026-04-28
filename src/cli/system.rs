use super::app_support::{render_with_value, resolve_optional_cli_installation, stable_services};
use super::output::system::{
    render_installation_health_report, render_installation_inspection, render_installation_scan,
    render_manifest_example, render_manifest_validation, render_runtime_diagnostics,
};
use super::{InstallTargetArgs, ManifestCommands, OptionalInstallTargetArgs};
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

mod request;

pub(in crate::cli) use request::{ManifestExampleResult, ManifestValidationResult};

use request::{
    build_inspect_installation_request, build_manifest_example_result,
    build_manifest_validation_result,
};

pub(super) fn handle_scan(json: bool, runtime: AppRuntime) -> AppResult<()> {
    let app = stable_services(runtime);
    render_with_value(json, || app.scan_installations(), render_installation_scan)
}

pub(super) fn handle_runtime(
    json: bool,
    runtime: AppRuntime,
    install_target: OptionalInstallTargetArgs,
) -> AppResult<()> {
    let app = stable_services(runtime);
    let installation = resolve_optional_cli_installation(&app, install_target)?;
    render_with_value(
        json,
        || match installation {
            Some(installation) => app.runtime_diagnostics_for_installation(installation),
            None => Ok(app.runtime_diagnostics()),
        },
        render_runtime_diagnostics,
    )
}

pub(super) fn handle_inspect(
    json: bool,
    runtime: AppRuntime,
    install_target: InstallTargetArgs,
) -> AppResult<()> {
    let app = stable_services(runtime);
    render_with_value(
        json,
        || app.inspect_installation(build_inspect_installation_request(install_target)),
        render_installation_inspection,
    )
}

pub(super) fn handle_doctor(
    json: bool,
    runtime: AppRuntime,
    install_target: InstallTargetArgs,
) -> AppResult<()> {
    let app = stable_services(runtime);
    render_with_value(
        json,
        || app.inspect_installation(build_inspect_installation_request(install_target)),
        |inspection| render_installation_health_report(&inspection.health),
    )
}

pub(super) fn handle_manifest_command(
    json: bool,
    runtime: AppRuntime,
    command: ManifestCommands,
) -> AppResult<()> {
    match command {
        ManifestCommands::Example => {
            render_with_value(json, build_manifest_example_result, render_manifest_example)?
        }
        ManifestCommands::Validate { file } => render_with_value(
            json,
            || build_manifest_validation_result(file, &runtime),
            render_manifest_validation,
        )?,
    }

    Ok(())
}
