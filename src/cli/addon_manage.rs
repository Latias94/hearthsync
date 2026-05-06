use super::app_support::{
    extended_services, render_with_fallible_installation, render_with_installation,
    render_with_installation_task_result, stable_services,
};
use super::output::addon::{
    render_addon_adopt, render_addon_install, render_addon_inventory, render_addon_relink,
    render_addon_remove, render_addon_search_catalog, render_addon_update,
};
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

mod request;

use request::{
    build_adopt_addons_request, build_install_addon_request, build_list_addons_request,
    build_relink_addon_request, build_remove_addons_request, build_search_addons_request,
    build_update_addons_request,
};

pub(super) fn handle_addon_search(
    json: bool,
    runtime: AppRuntime,
    install_target: crate::cli::InstallTargetArgs,
    query: String,
    limit: usize,
    provider: Option<String>,
) -> AppResult<()> {
    if provider.is_none() {
        let app = extended_services(runtime);
        return render_with_fallible_installation(
            json,
            app.stable(),
            install_target,
            |installation| installation.into_domain(),
            |installation| app.search_community_addon_index(query.clone(), limit, installation),
            super::output::addon::render_addon_index_search,
        );
    }

    let app = stable_services(runtime);

    render_with_installation(
        json,
        &app,
        install_target,
        |installation| build_search_addons_request(installation, query, limit, provider),
        |request| app.search_addons(request),
        render_addon_search_catalog,
    )
}

pub(super) fn handle_addon_list(
    json: bool,
    runtime: AppRuntime,
    install_target: crate::cli::InstallTargetArgs,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_installation(
        json,
        &app,
        install_target,
        build_list_addons_request,
        |request| app.list_addons(request),
        render_addon_inventory,
    )
}

pub(super) fn handle_addon_adopt(
    json: bool,
    runtime: AppRuntime,
    install_target: crate::cli::InstallTargetArgs,
    addon_directories: Vec<String>,
    package_id: Option<String>,
    archive_output: Option<std::path::PathBuf>,
    dry_run: bool,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_installation(
        json,
        &app,
        install_target,
        |installation| {
            build_adopt_addons_request(
                installation,
                addon_directories,
                package_id,
                archive_output,
                dry_run,
            )
        },
        |request| app.adopt_addons(request),
        render_addon_adopt,
    )
}

pub(super) fn handle_addon_relink(
    json: bool,
    runtime: AppRuntime,
    install_target: crate::cli::InstallTargetArgs,
    name: String,
    source: String,
    dry_run: bool,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_installation(
        json,
        &app,
        install_target,
        |installation| build_relink_addon_request(installation, name, source, dry_run),
        |request| app.relink_addon(request),
        render_addon_relink,
    )
}

pub(super) fn handle_addon_install(
    json: bool,
    runtime: AppRuntime,
    install_target: crate::cli::InstallTargetArgs,
    source: String,
    dry_run: bool,
    backup_output: Option<std::path::PathBuf>,
    replace_existing: bool,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_installation_task_result(
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
    .map(|_| ())
}

pub(super) fn handle_addon_update(
    json: bool,
    runtime: AppRuntime,
    install_target: crate::cli::InstallTargetArgs,
    name: Option<String>,
    dry_run: bool,
    backup_output: Option<std::path::PathBuf>,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_installation_task_result(
        json,
        &app,
        install_target,
        |installation| build_update_addons_request(installation, name, dry_run, backup_output),
        |request| app.update_addons(request),
        render_addon_update,
    )
    .map(|_| ())
}

pub(super) fn handle_addon_remove(
    json: bool,
    runtime: AppRuntime,
    install_target: crate::cli::InstallTargetArgs,
    name: String,
    dry_run: bool,
    backup_output: Option<std::path::PathBuf>,
) -> AppResult<()> {
    let app = stable_services(runtime);

    render_with_installation_task_result(
        json,
        &app,
        install_target,
        |installation| build_remove_addons_request(installation, name, dry_run, backup_output),
        |request| app.remove_addons(request),
        render_addon_remove,
    )
    .map(|_| ())
}
