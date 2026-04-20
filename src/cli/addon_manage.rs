use super::app_support::{render_with_installation, stable_services};
use super::output::addon::{
    render_addon_install, render_addon_inventory, render_addon_remove, render_addon_search_catalog,
    render_addon_update,
};
use crate::core::error::AppResult;

mod request;

use request::{
    build_install_addon_request, build_list_addons_request, build_remove_addons_request,
    build_search_addons_request, build_update_addons_request,
};

pub(super) fn handle_addon_search(
    json: bool,
    install_target: crate::cli::InstallTargetArgs,
    query: String,
    limit: usize,
) -> AppResult<()> {
    let app = stable_services();

    render_with_installation(
        json,
        &app,
        install_target,
        |installation| build_search_addons_request(installation, query, limit),
        |request| app.search_addons(request),
        render_addon_search_catalog,
    )
}

pub(super) fn handle_addon_list(
    json: bool,
    install_target: crate::cli::InstallTargetArgs,
) -> AppResult<()> {
    let app = stable_services();

    render_with_installation(
        json,
        &app,
        install_target,
        build_list_addons_request,
        |request| app.list_addons(request),
        render_addon_inventory,
    )
}

pub(super) fn handle_addon_install(
    json: bool,
    install_target: crate::cli::InstallTargetArgs,
    source: String,
    dry_run: bool,
    backup_output: Option<std::path::PathBuf>,
    replace_existing: bool,
) -> AppResult<()> {
    let app = stable_services();

    render_with_installation(
        json,
        &app,
        install_target,
        |installation| {
            build_install_addon_request(
                installation,
                source,
                dry_run,
                backup_output,
                replace_existing,
            )
        },
        |request| app.install_addon(request),
        render_addon_install,
    )
}

pub(super) fn handle_addon_update(
    json: bool,
    install_target: crate::cli::InstallTargetArgs,
    name: Option<String>,
    dry_run: bool,
    backup_output: Option<std::path::PathBuf>,
) -> AppResult<()> {
    let app = stable_services();

    render_with_installation(
        json,
        &app,
        install_target,
        |installation| build_update_addons_request(installation, name, dry_run, backup_output),
        |request| app.update_addons(request),
        render_addon_update,
    )
}

pub(super) fn handle_addon_remove(
    json: bool,
    install_target: crate::cli::InstallTargetArgs,
    name: String,
    dry_run: bool,
    backup_output: Option<std::path::PathBuf>,
) -> AppResult<()> {
    let app = stable_services();

    render_with_installation(
        json,
        &app,
        install_target,
        |installation| build_remove_addons_request(installation, name, dry_run, backup_output),
        |request| app.remove_addons(request),
        render_addon_remove,
    )
}
