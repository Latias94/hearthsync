use std::path::PathBuf;

use serde::Serialize;

use super::super::bundle::BundleResourcesResult;
use super::super::external_package::ExternalPackageAnalysisResult;
use super::entry::ConfigPackageEntryResult;
use super::source::ConfigPackageSourceKindResult;
use super::warning::{ConfigPackageSummaryResult, ConfigWarningResult};

#[derive(Debug, Clone, Serialize)]
pub struct ConfigInspectionResult {
    pub source_path: PathBuf,
    pub source_kind: ConfigPackageSourceKindResult,
    pub package_id: String,
    pub package_name: String,
    pub entry_count: usize,
    pub entries: Vec<ConfigPackageEntryResult>,
    pub resources: BundleResourcesResult,
    pub summary: ConfigPackageSummaryResult,
    pub warnings: Vec<ConfigWarningResult>,
}

impl ConfigInspectionResult {
    pub(crate) fn from_external(value: ExternalPackageAnalysisResult) -> Self {
        Self {
            source_path: value.source_path,
            source_kind: ConfigPackageSourceKindResult::from_external(value.source_kind),
            package_id: value.package_id,
            package_name: value.package_name,
            entry_count: value.entry_count,
            entries: value
                .entries
                .into_iter()
                .map(ConfigPackageEntryResult::from_external)
                .collect(),
            resources: value.resources,
            summary: ConfigPackageSummaryResult::from_external(value.summary),
            warnings: value
                .warnings
                .into_iter()
                .map(ConfigWarningResult::from_external)
                .collect(),
        }
    }
}
