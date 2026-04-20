use super::app_support::{render_with_value, stable_services};
use super::output::{
    render_installation_health_report, render_installation_inspection, render_installation_scan,
};
use super::{InstallTargetArgs, ManifestCommands};
use crate::core::error::AppResult;
use crate::core::manifest::{example_manifest, load_manifest};

mod request;

use request::build_inspect_installation_request;

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
            print!("{}", example_manifest()?);
        }
        ManifestCommands::Validate { file } => {
            let manifest = load_manifest(&file)?;
            manifest.validate()?;
            if json {
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
    }

    Ok(())
}
