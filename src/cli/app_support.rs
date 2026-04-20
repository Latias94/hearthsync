use super::mapping::resolve_apply_mappings;
use super::{ApplyMappingArgs, InstallTargetArgs};
use crate::core::app::{
    BundleApplyMappingsValue, ExtendedAppServices, ResolveInstallationRequest,
    ResolvedInstallationValue, StableAppServices,
};
use crate::core::error::AppResult;

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
