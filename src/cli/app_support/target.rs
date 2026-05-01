use crate::cli::mapping::resolve_apply_mappings;
use crate::cli::{ApplyMappingArgs, InstallTargetArgs, OptionalInstallTargetArgs};
use crate::core::app::{
    AppRuntime, BundleApplyMappingsValue, ResolveInstallationRequest, ResolvedInstallationValue,
    StableAppServices,
};
use crate::core::error::AppResult;

pub(in crate::cli) struct ResolvedCliApplyTarget {
    pub(in crate::cli) installation: ResolvedInstallationValue,
    pub(in crate::cli) apply_mappings: BundleApplyMappingsValue,
}

#[derive(Clone, Copy)]
pub(in crate::cli) struct CliAppContext<'a> {
    pub(in crate::cli) services: &'a StableAppServices,
    pub(in crate::cli) runtime: &'a AppRuntime,
}

impl<'a> CliAppContext<'a> {
    pub(in crate::cli) fn new(services: &'a StableAppServices, runtime: &'a AppRuntime) -> Self {
        Self { services, runtime }
    }
}

pub(in crate::cli) fn resolve_cli_installation(
    services: &StableAppServices,
    install_target: InstallTargetArgs,
) -> AppResult<ResolvedInstallationValue> {
    services.resolve_installation(ResolveInstallationRequest {
        path: install_target.install,
        flavor: install_target.flavor.map(Into::into),
    })
}

pub(in crate::cli) fn resolve_optional_cli_installation(
    services: &StableAppServices,
    install_target: OptionalInstallTargetArgs,
) -> AppResult<Option<ResolvedInstallationValue>> {
    let Some(path) = install_target.install else {
        return Ok(None);
    };

    services
        .resolve_installation(ResolveInstallationRequest {
            path,
            flavor: install_target.flavor.map(Into::into),
        })
        .map(Some)
}

pub(in crate::cli) fn resolve_cli_apply_target(
    context: CliAppContext<'_>,
    install_target: InstallTargetArgs,
    apply_mapping: ApplyMappingArgs,
) -> AppResult<ResolvedCliApplyTarget> {
    Ok(ResolvedCliApplyTarget {
        installation: resolve_cli_installation(context.services, install_target)?,
        apply_mappings: resolve_apply_mappings(apply_mapping, context.runtime)?,
    })
}
