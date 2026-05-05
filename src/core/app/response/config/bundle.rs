use serde::Serialize;

use super::super::bundle::{BundleManifestResult, CreatedBundleResult};
use super::super::external_package::{ExternalPackageBundleHandle, ExternalPackageBundleResult};
use super::inspection::ConfigInspectionResult;

#[derive(Debug, Clone, Serialize)]
pub struct ConfigBundleResult {
    pub inspection: ConfigInspectionResult,
    pub manifest: BundleManifestResult,
    pub bundle: CreatedBundleResult,
}

impl ConfigBundleResult {
    fn from_external(value: ExternalPackageBundleResult) -> Self {
        Self {
            inspection: ConfigInspectionResult::from_external(value.analysis),
            manifest: value.manifest,
            bundle: value.bundle,
        }
    }
}

#[derive(Debug)]
pub struct ConfigBundleHandle {
    result: ConfigBundleResult,
    _external: ExternalPackageBundleHandle,
}

impl ConfigBundleHandle {
    pub(crate) fn from_external(value: ExternalPackageBundleHandle) -> Self {
        let result = ConfigBundleResult::from_external(value.as_ref().clone());

        Self {
            result,
            _external: value,
        }
    }
}

impl AsRef<ConfigBundleResult> for ConfigBundleHandle {
    fn as_ref(&self) -> &ConfigBundleResult {
        &self.result
    }
}
