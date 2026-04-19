use std::path::PathBuf;

use super::app_support::stable_services;
use super::output::render;
use super::{FlavorArg, ManifestCommands};
use crate::core::app::{InspectInstallationRequest, InstallationHealthResult};
use crate::core::error::AppResult;
use crate::core::manifest::{example_manifest, load_manifest};

pub(super) fn handle_scan(json: bool) -> AppResult<()> {
    let app = stable_services();
    let installations = app.scan_installations()?;
    render(json, &installations, |item| {
        if item.installations.is_empty() {
            "No World of Warcraft installations detected.".to_string()
        } else {
            let mut lines = vec![format!(
                "Detected {} installation(s):",
                item.installation_count
            )];
            for installation in &item.installations {
                lines.push(format!(
                    "- {} => {}",
                    installation.flavor.as_str(),
                    installation.flavor_root.display()
                ));
            }
            lines.join("\n")
        }
    })
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
    render(json, &inspection, |item| {
        format!(
            "Flavor: {}\nProduct root: {}\nFlavor root: {}\nAddOns: {}\nWTF: {}\nFonts: {}\nHealth: {}",
            item.installation.flavor.as_str(),
            item.product_root.display(),
            item.installation.flavor_root.display(),
            item.installation.addon_dir.display(),
            item.installation.wtf_dir.display(),
            item.installation.fonts_dir.display(),
            item.health.status_label
        )
    })
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
    render(json, &inspection.health, format_installation_health_report)
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

fn format_installation_health_report(health: &InstallationHealthResult) -> String {
    let mut lines = vec![format!("Status: {}", health.status_label)];

    if health.missing_paths.is_empty() {
        lines.push("Missing required paths: none".to_string());
    } else {
        lines.push("Missing required paths:".to_string());
        for path in &health.missing_paths {
            lines.push(format!("- {}", path.display()));
        }
    }

    if health.warnings.is_empty() {
        lines.push("Warnings: none".to_string());
    } else {
        lines.push("Warnings:".to_string());
        for warning in &health.warnings {
            lines.push(format!("- {warning}"));
        }
    }

    lines.join("\n")
}
