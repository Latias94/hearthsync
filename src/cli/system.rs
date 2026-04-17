use std::path::PathBuf;

use super::output::render;
use super::{FlavorArg, ManifestCommands};
use crate::core::app::HearthSyncApp;
use crate::core::error::AppResult;
use crate::core::manifest::{example_manifest, load_manifest};

pub(super) fn handle_scan(json: bool) -> AppResult<()> {
    let service = HearthSyncApp::new().installations();
    let installations = service.scan()?;
    render(json, &installations, |items| {
        if items.is_empty() {
            "No World of Warcraft installations detected.".to_string()
        } else {
            let mut lines = vec![format!("Detected {} installation(s):", items.len())];
            for item in items {
                lines.push(format!(
                    "- {} => {}",
                    item.flavor.as_str(),
                    item.flavor_root.display()
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
    let service = HearthSyncApp::new().installations();
    let inspection = service.inspect(&install, flavor.map(Into::into))?;
    render(json, &inspection, |item| {
        format!(
            "Flavor: {}\nProduct root: {}\nFlavor root: {}\nAddOns: {}\nWTF: {}\nFonts: {}\nHealth: {}",
            item.installation.flavor.as_str(),
            item.product_root.display(),
            item.installation.flavor_root.display(),
            item.installation.addon_dir.display(),
            item.installation.wtf_dir.display(),
            item.installation.fonts_dir.display(),
            item.health.summary()
        )
    })
}

pub(super) fn handle_doctor(
    json: bool,
    install: PathBuf,
    flavor: Option<FlavorArg>,
) -> AppResult<()> {
    let service = HearthSyncApp::new().installations();
    let inspection = service.inspect(&install, flavor.map(Into::into))?;
    render(json, &inspection.health, |health| health.to_report())
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
