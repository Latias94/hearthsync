use std::path::PathBuf;

use super::FlavorArg;
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
    path: PathBuf,
    flavor: Option<FlavorArg>,
) -> AppResult<ResolvedInstallationValue> {
    services.resolve_installation(ResolveInstallationRequest {
        path,
        flavor: flavor.map(Into::into),
    })
}
