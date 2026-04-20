use super::app_support::{render_with_apply_target, stable_services};
use super::output::bundle::{render_bundle_apply, render_bundle_apply_plan};
use super::{ApplyMappingArgs, InstallTargetArgs};
use crate::core::error::AppResult;

mod request;

use request::{build_apply_bundle_request, build_plan_bundle_apply_request};

pub(super) fn handle_bundle_plan(
    json: bool,
    bundle: std::path::PathBuf,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
) -> AppResult<()> {
    let app = stable_services();

    render_with_apply_target(
        json,
        &app,
        install_target,
        apply_mapping,
        |target| {
            build_plan_bundle_apply_request(bundle, target.installation, target.apply_mappings)
        },
        |request| app.plan_bundle_apply(request),
        render_bundle_apply_plan,
    )
}

pub(super) fn handle_bundle_unpack(
    json: bool,
    bundle: std::path::PathBuf,
    install_target: InstallTargetArgs,
    dry_run: bool,
    backup_output: Option<std::path::PathBuf>,
    apply_mapping: ApplyMappingArgs,
) -> AppResult<()> {
    let app = stable_services();

    render_with_apply_target(
        json,
        &app,
        install_target,
        apply_mapping,
        |target| {
            build_apply_bundle_request(
                bundle,
                target.installation,
                dry_run,
                backup_output,
                target.apply_mappings,
            )
        },
        |request| app.apply_bundle(request),
        render_bundle_apply,
    )
}
