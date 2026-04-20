use super::super::apply_model::{PreparedApplySource, PreparedBundleApply};
use super::super::planner::prepare_apply_from_source;
use super::super::types::BundleApplyMappings;
use super::analyze::analyze_external_package;
use super::manifest::build_external_manifest;
use super::normalized::{
    build_external_package_entry_source_map, validate_unique_normalized_paths,
};
use super::types::{
    AnalyzeExternalPackageRequest, CreateExternalPackageBundleRequest, ExternalPackageAnalysis,
};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::manifest::BundleManifest;

#[derive(Debug)]
pub(in crate::core::bundle::external_package) struct PreparedExternalPackageApply {
    pub(in crate::core::bundle::external_package) analysis: ExternalPackageAnalysis,
    pub(in crate::core::bundle::external_package) prepared_apply: PreparedBundleApply,
}

pub(super) fn prepare_external_package_artifacts(
    request: &CreateExternalPackageBundleRequest,
) -> AppResult<(ExternalPackageAnalysis, BundleManifest)> {
    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: request.source_path.clone(),
    })?;
    validate_unique_normalized_paths(&analysis)?;

    let manifest = build_external_manifest(&analysis, request);
    manifest.validate()?;

    Ok((analysis, manifest))
}

pub(in crate::core::bundle::external_package) fn prepare_external_package_apply(
    external_package: CreateExternalPackageBundleRequest,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<PreparedExternalPackageApply> {
    let (analysis, manifest) = prepare_external_package_artifacts(&external_package)?;
    let entry_source_map = build_external_package_entry_source_map(&analysis)?;
    let source_path = analysis.source_path.clone();
    let prepared_apply = prepare_apply_from_source(
        &source_path,
        installation,
        manifest,
        apply_mappings,
        PreparedApplySource::ExternalPackage {
            source_path: source_path.clone(),
            source_kind: analysis.source_kind,
            entry_source_map,
        },
    )?;

    Ok(PreparedExternalPackageApply {
        analysis,
        prepared_apply,
    })
}
