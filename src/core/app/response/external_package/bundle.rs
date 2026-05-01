use serde::Serialize;

use crate::core::bundle::PreparedExternalPackageBundle as DomainPreparedExternalPackageBundle;

use super::super::bundle::{BundleManifestResult, CreatedBundleResult};
use super::analysis::ExternalPackageAnalysisResult;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageBundleResult {
    pub analysis: ExternalPackageAnalysisResult,
    pub manifest: BundleManifestResult,
    pub bundle: CreatedBundleResult,
}

#[derive(Debug)]
pub struct ExternalPackageBundleHandle {
    result: ExternalPackageBundleResult,
    _prepared: DomainPreparedExternalPackageBundle,
}

impl ExternalPackageBundleHandle {
    pub(crate) fn from_domain(value: DomainPreparedExternalPackageBundle) -> Self {
        let result = ExternalPackageBundleResult {
            analysis: ExternalPackageAnalysisResult::from_domain(value.analysis.clone()),
            manifest: BundleManifestResult::from_domain(value.manifest.clone()),
            bundle: CreatedBundleResult::from_domain(value.bundle.clone()),
        };

        Self {
            result,
            _prepared: value,
        }
    }
}

impl AsRef<ExternalPackageBundleResult> for ExternalPackageBundleHandle {
    fn as_ref(&self) -> &ExternalPackageBundleResult {
        &self.result
    }
}
