use std::path::PathBuf;

use crate::core::app::{AppRuntime, WowFlavorValue};
use crate::core::error::AppResult;
use crate::core::install::{
    DetectedFlavorInstallation, ProductInstallInspection, inspect_installation_on_host,
    resolve_installation_on_host,
};

#[derive(Debug, Clone)]
pub struct InspectInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavorValue>,
}

impl InspectInstallationRequest {
    pub(crate) fn inspect_with_runtime(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<ProductInstallInspection> {
        inspect_installation_on_host(
            &self.path,
            self.flavor.map(Into::into),
            runtime.host_platform().into(),
        )
    }
}

#[derive(Debug, Clone)]
pub struct ResolveInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavorValue>,
}

impl ResolveInstallationRequest {
    pub(crate) fn resolve_with_runtime(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DetectedFlavorInstallation> {
        resolve_installation_on_host(
            &self.path,
            self.flavor.map(Into::into),
            runtime.host_platform().into(),
        )
    }
}
