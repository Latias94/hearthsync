use super::BundleCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::output::{render, render_bundle_archive_created, render_bundle_archive_inspection};
use crate::core::app::{BundleManifestValue, InspectBundleRequest, PackBundleAppRequest};
use crate::core::error::{AppError, AppResult};
use crate::core::manifest::load_manifest;

pub(super) fn handle_bundle_archive_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        BundleCommands::Pack {
            install,
            flavor,
            manifest,
            output,
        } => {
            let installation = resolve_cli_installation(&app, install, flavor)?;
            let manifest_base_dir = manifest.parent().map(|path| path.to_path_buf());
            let manifest = BundleManifestValue::from_domain(load_manifest(&manifest)?);
            let bundle = app.pack_bundle(PackBundleAppRequest {
                installation,
                manifest,
                output_path: output,
                manifest_base_dir,
            })?;
            render(json, &bundle, render_bundle_archive_created)?;
        }
        BundleCommands::Inspect { bundle } => {
            let inspection = app.inspect_bundle(InspectBundleRequest {
                bundle_path: bundle,
            })?;
            render(json, &inspection, render_bundle_archive_inspection)?;
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
