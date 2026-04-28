use std::path::PathBuf;

use super::resolve_app_input_path;
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
        let path = resolve_app_input_path(runtime, self.path, "installation path")?;
        inspect_installation_on_host(
            &path,
            self.flavor.map(WowFlavorValue::into_domain),
            runtime.host_platform().into_domain(),
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
        let path = resolve_app_input_path(runtime, self.path, "installation path")?;
        resolve_installation_on_host(
            &path,
            self.flavor.map(WowFlavorValue::into_domain),
            runtime.host_platform().into_domain(),
        )
    }
}
