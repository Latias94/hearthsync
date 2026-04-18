use super::BundleCommands;
use super::output::render;
use crate::core::app::{
    HearthSyncApp, InspectBundleRequest, PackBundleAppRequest, ResolveInstallationRequest,
};
use crate::core::error::{AppError, AppResult};
use crate::core::manifest::load_manifest;

pub(super) fn handle_bundle_archive_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = HearthSyncApp::new();
    let service = app.bundles();
    let installation_service = app.installations();

    match command {
        BundleCommands::Pack {
            install,
            flavor,
            manifest,
            output,
        } => {
            let installation = installation_service.resolve(ResolveInstallationRequest {
                path: install,
                flavor: flavor.map(Into::into),
            })?;
            let manifest_base_dir = manifest.parent().map(|path| path.to_path_buf());
            let manifest = load_manifest(&manifest)?.into();
            let bundle = service.pack(PackBundleAppRequest {
                installation,
                manifest,
                output_path: output,
                manifest_base_dir,
            })?;
            render(json, &bundle, |item| {
                format!(
                    "Created bundle: {}\nArchived files: {}\nPackage: {}",
                    item.archive_path.display(),
                    item.archived_files,
                    item.manifest.package.name
                )
            })?;
        }
        BundleCommands::Inspect { bundle } => {
            let inspection = service.inspect(InspectBundleRequest {
                bundle_path: bundle,
            })?;
            render(json, &inspection, |item| {
                let characters = item
                    .resources
                    .wtf_characters
                    .iter()
                    .map(|character| {
                        format!(
                            "{}/{}/{}",
                            character
                                .source_account
                                .as_deref()
                                .unwrap_or("<unknown-account>"),
                            character.source_server,
                            character.source_character
                        )
                    })
                    .collect::<Vec<_>>();
                format!(
                    "Bundle: {}\nPackage: {}\nSource flavor: {}\nFiles: {}\nAddOns: {}\nWTF common: {}\nWTF characters: {}\nFonts: {}\nInterface assets: {}\nCharacters: {}",
                    item.archive_path.display(),
                    item.package.name,
                    item.source.flavor.as_str(),
                    item.entries.total_files,
                    item.entries.addons,
                    item.entries.wtf_common,
                    item.entries.wtf_characters,
                    item.entries.fonts,
                    item.entries.interface_assets,
                    if characters.is_empty() {
                        "none".to_string()
                    } else {
                        characters.join(", ")
                    }
                )
            })?;
        }
        _ => {
            return Err(AppError::Validation(
                "internal CLI routing error: bundle archive handler received unexpected command"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
