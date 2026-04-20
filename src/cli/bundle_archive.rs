use super::BundleCommands;
use super::app_support::{render_with_installation, render_with_value, stable_services};
use super::output::{render_bundle_archive_created, render_bundle_archive_inspection};
use crate::core::error::{AppError, AppResult};

mod request;

use request::{build_inspect_bundle_request, build_pack_bundle_request};

pub(super) fn handle_bundle_archive_command(json: bool, command: BundleCommands) -> AppResult<()> {
    let app = stable_services();

    match command {
        BundleCommands::Pack {
            install_target,
            manifest,
            output,
        } => render_with_installation(
            json,
            &app,
            install_target,
            |installation| build_pack_bundle_request(installation, manifest, output),
            |request| app.pack_bundle(request?),
            render_bundle_archive_created,
        )?,
        BundleCommands::Inspect { bundle } => render_with_value(
            json,
            || app.inspect_bundle(build_inspect_bundle_request(bundle)),
            render_bundle_archive_inspection,
        )?,
        _ => {
            return Err(AppError::Validation(
                "internal CLI routing error: bundle archive handler received unexpected command"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
