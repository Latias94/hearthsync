use super::InstallTargetArgs;
use crate::core::app::{
    ExtendedAppServices, ResolveInstallationRequest, ResolvedInstallationValue, StableAppServices,
};
use crate::core::error::AppResult;

pub(super) fn stable_services() -> StableAppServices {
    StableAppServices::new()
}

pub(super) fn extended_services() -> ExtendedAppServices {
    ExtendedAppServices::new()
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
