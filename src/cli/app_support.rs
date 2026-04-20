use super::mapping::resolve_apply_mappings;
use super::output::render;
use super::{ApplyMappingArgs, InstallTargetArgs};
use crate::core::app::{
    BundleApplyMappingsValue, ExtendedAppServices, ResolveInstallationRequest,
    ResolvedInstallationValue, StableAppServices,
};
use crate::core::error::AppResult;
use serde::Serialize;

pub(super) fn stable_services() -> StableAppServices {
    StableAppServices::new()
}

pub(super) fn extended_services() -> ExtendedAppServices {
    ExtendedAppServices::new()
}

pub(super) struct ResolvedCliApplyTarget {
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

pub(super) fn resolve_cli_installation(
    services: &StableAppServices,
    install_target: InstallTargetArgs,
) -> AppResult<ResolvedInstallationValue> {
    services.resolve_installation(ResolveInstallationRequest {
        path: install_target.install,
        flavor: install_target.flavor.map(Into::into),
    })
}

pub(super) fn resolve_cli_apply_target(
    services: &StableAppServices,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
) -> AppResult<ResolvedCliApplyTarget> {
    Ok(ResolvedCliApplyTarget {
        installation: resolve_cli_installation(services, install_target)?,
        apply_mappings: resolve_apply_mappings(apply_mapping)?,
    })
}

pub(super) fn render_with_installation<Req, Res, Build, Invoke, Format>(
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

pub(super) fn render_with_fallible_installation<Req, Res, Build, Invoke, Format>(
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

pub(super) fn render_with_value<Res, Invoke, Format>(
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

pub(super) fn render_with_apply_target<Req, Res, Build, Invoke, Format>(
    json: bool,
    services: &StableAppServices,
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
    let target = resolve_cli_apply_target(services, install_target, apply_mapping)?;
    let result = invoke(build_request(target))?;
    render(json, &result, text_renderer)
}
