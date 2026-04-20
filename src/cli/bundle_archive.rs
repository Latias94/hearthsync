use super::BundleCommands;
use super::app_support::{resolve_cli_installation, stable_services};
use super::output::{render, render_bundle_archive_created, render_bundle_archive_inspection};
use crate::core::error::{AppError, AppResult};

mod request;

use request::{build_inspect_bundle_request, build_pack_bundle_request};

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
            let bundle =
                app.pack_bundle(build_pack_bundle_request(installation, manifest, output)?)?;
            render(json, &bundle, render_bundle_archive_created)?;
        }
        BundleCommands::Inspect { bundle } => {
            let inspection = app.inspect_bundle(build_inspect_bundle_request(bundle))?;
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
