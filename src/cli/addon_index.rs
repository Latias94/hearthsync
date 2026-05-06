use super::AddonIndexCommands;
use super::app_support::{
    extended_services, render_with_installation, render_with_installation_task_result,
    render_with_value, resolve_cli_installation,
};
use super::output::addon::{
    render_addon_index_attach, render_addon_index_inspection, render_addon_index_install,
    render_addon_index_relink, render_addon_index_scaffold, render_addon_index_search,
    render_addon_index_suggestion, render_addon_index_update, render_addon_index_validation,
};
use super::output::render;
use crate::core::app::AppRuntime;
use crate::core::error::{AppError, AppResult};

mod request;

use request::{
    build_attach_addon_index_request, build_inspect_addon_index_request,
    build_install_addon_index_request, build_relink_addon_index_request,
    build_scaffold_addon_index_request, build_search_addon_index_request,
    build_suggest_addon_index_request, build_update_addon_index_request,
};

pub(super) fn handle_addon_index_command(
    json: bool,
    runtime: AppRuntime,
    command: AddonIndexCommands,
) -> AppResult<()> {
    let app = extended_services(runtime);

    match command {
        AddonIndexCommands::Inspect { file } => render_with_value(
            json,
            || app.inspect_addon_index(build_inspect_addon_index_request(file)),
            render_addon_index_inspection,
        )?,
        AddonIndexCommands::Search { file, query, limit } => render_with_value(
            json,
            || app.search_addon_index(build_search_addon_index_request(file, query, limit)),
            render_addon_index_search,
        )?,
        AddonIndexCommands::Validate { file } => {
            let result = app.validate_addon_index(build_inspect_addon_index_request(file))?;
            render(json, &result, render_addon_index_validation)?;
            if !result.valid {
                return Err(AppError::Validation(format!(
                    "addon index validation failed with {} blocking warning(s) and {} advisory warning(s)",
                    result.blocking_warning_count, result.advisory_warning_count
                )));
            }
        }
        AddonIndexCommands::Scaffold {
            install_target,
            file,
            index_name,
            description,
            name,
            overwrite,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| {
                build_scaffold_addon_index_request(
                    installation,
                    file,
                    index_name,
                    description,
                    name,
                    overwrite,
                )
            },
            |request| app.scaffold_addon_index(request),
            render_addon_index_scaffold,
        )?,
        AddonIndexCommands::Suggest {
            install_target,
            file,
            name,
        } => render_with_installation(
            json,
            app.stable(),
            install_target,
            |installation| build_suggest_addon_index_request(installation, file, name),
            |request| app.suggest_addon_index(request),
            render_addon_index_suggestion,
        )?,
        AddonIndexCommands::Attach {
            install_target,
            file,
            name,
            dry_run,
            apply_ready_only,
        } => {
            let installation = resolve_cli_installation(app.stable(), install_target)?;
            let run = app.attach_addon_index(build_attach_addon_index_request(
                installation,
                file,
                name,
                dry_run,
                apply_ready_only,
            ))?;
            let result = run.result;
            render(json, &result, render_addon_index_attach)?;
            if !dry_run && !result.ready && !result.applied {
                return Err(AppError::Validation(format!(
                    "addon index attach is blocked by {} package(s); no registry changes were written",
                    result.blocked_package_count
                )));
            }
        }
        AddonIndexCommands::Install {
            install_target,
            file,
            name,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            render_with_installation_task_result(
                json,
                app.stable(),
                install_target,
                |installation| {
                    build_install_addon_index_request(
                        installation,
                        file,
                        name,
                        dry_run,
                        backup_output,
                        replace_existing,
                    )
                },
                |request| app.install_addon_index(request),
                render_addon_index_install,
            )?;
        }
        AddonIndexCommands::Update {
            install_target,
            file,
            name,
            dry_run,
            backup_output,
        } => {
            render_with_installation_task_result(
                json,
                app.stable(),
                install_target,
                |installation| {
                    build_update_addon_index_request(
                        installation,
                        file,
                        name,
                        dry_run,
                        backup_output,
                    )
                },
                |request| app.update_addon_index(request),
                render_addon_index_update,
            )?;
        }
        AddonIndexCommands::Relink {
            install_target,
            file,
            name,
            target,
            dry_run,
        } => {
            render_with_installation_task_result(
                json,
                app.stable(),
                install_target,
                |installation| {
                    build_relink_addon_index_request(installation, file, name, target, dry_run)
                },
                |request| app.relink_addon_index(request),
                render_addon_index_relink,
            )?;
        }
    }

    Ok(())
}
