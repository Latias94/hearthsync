use serde::Serialize;

use crate::cli::output::render;
use crate::cli::{ApplyMappingArgs, InstallTargetArgs};
use crate::core::app::{ResolvedInstallationValue, StableAppServices, TaskRun};
use crate::core::error::AppResult;

use super::target::{
    CliAppContext, ResolvedCliApplyTarget, resolve_cli_apply_target, resolve_cli_installation,
};

pub(in crate::cli) fn render_with_installation<Req, Res, Build, Invoke, Format>(
    json: bool,
    services: &StableAppServices,
    install_target: InstallTargetArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Build: FnOnce(ResolvedInstallationValue) -> Req,
    Invoke: FnOnce(Req) -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let installation = resolve_cli_installation(services, install_target)?;
    let result = invoke(build_request(installation))?;
    render(json, &result, text_renderer)
}

pub(in crate::cli) fn render_with_fallible_installation<Req, Res, Build, Invoke, Format>(
    json: bool,
    services: &StableAppServices,
    install_target: InstallTargetArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Build: FnOnce(ResolvedInstallationValue) -> AppResult<Req>,
    Invoke: FnOnce(Req) -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let installation = resolve_cli_installation(services, install_target)?;
    let request = build_request(installation)?;
    let result = invoke(request)?;
    render(json, &result, text_renderer)
}

pub(in crate::cli) fn render_with_value<Res, Invoke, Format>(
    json: bool,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Invoke: FnOnce() -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let result = invoke()?;
    render(json, &result, text_renderer)
}

pub(in crate::cli) fn render_task_result<Res, Invoke, Format>(
    json: bool,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<TaskRun<Res>>
where
    Res: Serialize,
    Invoke: FnOnce() -> AppResult<TaskRun<Res>>,
    Format: FnOnce(&Res) -> String,
{
    let run = invoke()?;
    render(json, &run.result, text_renderer)?;
    Ok(run)
}

pub(in crate::cli) fn render_with_installation_task_result<Req, Res, Build, Invoke, Format>(
    json: bool,
    services: &StableAppServices,
    install_target: InstallTargetArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<TaskRun<Res>>
where
    Res: Serialize,
    Build: FnOnce(ResolvedInstallationValue) -> Req,
    Invoke: FnOnce(Req) -> AppResult<TaskRun<Res>>,
    Format: FnOnce(&Res) -> String,
{
    let installation = resolve_cli_installation(services, install_target)?;
    render_task_result(json, || invoke(build_request(installation)), text_renderer)
}

pub(in crate::cli) fn render_with_apply_target<Req, Res, Build, Invoke, Format>(
    json: bool,
    context: CliAppContext<'_>,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<()>
where
    Res: Serialize,
    Build: FnOnce(ResolvedCliApplyTarget) -> Req,
    Invoke: FnOnce(Req) -> AppResult<Res>,
    Format: FnOnce(&Res) -> String,
{
    let target = resolve_cli_apply_target(context, install_target, apply_mapping)?;
    let result = invoke(build_request(target))?;
    render(json, &result, text_renderer)
}

pub(in crate::cli) fn render_with_apply_target_task_result<Req, Res, Build, Invoke, Format>(
    json: bool,
    context: CliAppContext<'_>,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
    build_request: Build,
    invoke: Invoke,
    text_renderer: Format,
) -> AppResult<TaskRun<Res>>
where
    Res: Serialize,
    Build: FnOnce(ResolvedCliApplyTarget) -> Req,
    Invoke: FnOnce(Req) -> AppResult<TaskRun<Res>>,
    Format: FnOnce(&Res) -> String,
{
    let target = resolve_cli_apply_target(context, install_target, apply_mapping)?;
    render_task_result(json, || invoke(build_request(target)), text_renderer)
}
