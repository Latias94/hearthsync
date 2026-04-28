use super::InstallTargetArgs;
use super::app_support::{render_with_fallible_installation, render_with_value, stable_services};
use super::output::bundle::{render_bundle_archive_created, render_bundle_archive_inspection};
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

mod request;

use request::{build_inspect_bundle_request, build_pack_bundle_request};

pub(super) fn handle_bundle_pack(
    json: bool,
    runtime: AppRuntime,
    install_target: InstallTargetArgs,
    manifest: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_fallible_installation(
        json,
        &app,
        install_target,
        |installation| build_pack_bundle_request(installation, manifest, output),
        |request| app.pack_bundle(request),
        render_bundle_archive_created,
    )
}

pub(super) fn handle_bundle_inspect(
    json: bool,
    runtime: AppRuntime,
    bundle: std::path::PathBuf,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_value(
        json,
        || app.inspect_bundle(build_inspect_bundle_request(bundle)),
        render_bundle_archive_inspection,
    )
}
