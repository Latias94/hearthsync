use std::path::PathBuf;

use serde::Serialize;

use crate::core::bundle::{
    ExternalPackageAnalysis as DomainExternalPackageAnalysis, ExternalPackageSourceKind,
};

use super::super::super::map_owned_vec;
use super::super::bundle::BundleResourcesResult;
use super::entry::ExternalPackageEntryResult;
use super::warning::{ExternalPackageSummaryResult, ExternalPackageWarningResult};

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageAnalysisResult {
    pub source_path: PathBuf,
    pub source_kind: ExternalPackageSourceKind,
    pub package_id: String,
    pub package_name: String,
    pub entry_count: usize,
    pub entries: Vec<ExternalPackageEntryResult>,
    pub resources: BundleResourcesResult,
    pub summary: ExternalPackageSummaryResult,
    pub warnings: Vec<ExternalPackageWarningResult>,
}

impl ExternalPackageAnalysisResult {
    pub(crate) fn from_domain(value: DomainExternalPackageAnalysis) -> Self {
        let entry_count = value.entries.len();

        Self {
            source_path: value.source_path,
            source_kind: value.source_kind,
            package_id: value.package_id,
            package_name: value.package_name,
            entry_count,
            entries: map_owned_vec(value.entries, ExternalPackageEntryResult::from_domain),
            resources: BundleResourcesResult::from_domain(value.resources),
            summary: ExternalPackageSummaryResult::from_domain(value.summary),
            warnings: map_owned_vec(value.warnings, ExternalPackageWarningResult::from_domain),
        }
    }
}
