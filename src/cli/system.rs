use std::path::PathBuf;

use super::app_support::stable_services;
use super::output::{
    render, render_installation_health_report, render_installation_inspection,
    render_installation_scan,
};
use super::{FlavorArg, ManifestCommands};
use crate::core::app::InspectInstallationRequest;
use crate::core::error::AppResult;
use crate::core::manifest::{example_manifest, load_manifest};

pub(super) fn handle_scan(json: bool) -> AppResult<()> {
    let app = stable_services();
    let installations = app.scan_installations()?;
    render(json, &installations, render_installation_scan)
}

pub(super) fn handle_inspect(
    json: bool,
    install: PathBuf,
    flavor: Option<FlavorArg>,
) -> AppResult<()> {
    let app = stable_services();
    let inspection = app.inspect_installation(InspectInstallationRequest {
        path: install,
        flavor: flavor.map(Into::into),
    })?;
    render(json, &inspection, render_installation_inspection)
}

pub(super) fn handle_doctor(
    json: bool,
    install: PathBuf,
    flavor: Option<FlavorArg>,
) -> AppResult<()> {
    let app = stable_services();
    let inspection = app.inspect_installation(InspectInstallationRequest {
        path: install,
        flavor: flavor.map(Into::into),
    })?;
    render(json, &inspection.health, render_installation_health_report)
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
