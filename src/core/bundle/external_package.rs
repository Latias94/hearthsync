use tempfile::tempdir;

mod analysis;
mod classify;
mod manifest;
mod materialize;
mod normalized;
mod projection;
mod source;
mod tasks;
mod types;

use super::*;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform};
use crate::core::manifest::BundleManifest;
use analysis::build_analysis;
use classify::classify_source_entries;
pub(crate) use manifest::author_package_apply_defaults;
use manifest::build_external_manifest;
use materialize::{create_staging_installation, materialize_analysis_to_installation};
use normalized::{build_external_package_entry_source_map, validate_unique_normalized_paths};
use projection::project_external_package_plan;
use source::{collect_source_entries, detect_source_kind};
pub use tasks::{
    analyze_external_package_task, apply_external_package, apply_external_package_task,
    plan_external_package_apply_task,
};
pub use types::{
    AnalyzeExternalPackageRequest, AppliedExternalPackage, ApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest, ExternalPackageAnalysis, ExternalPackageApplyPlan,
    ExternalPackageEntry, ExternalPackageSourceKind, ExternalPackageSummary,
    ExternalPackageWarning, ExternalPackageWarningCategory, ExternalPackageWarningCode,
    ExternalPackageWarningGroup, PlanExternalPackageApplyRequest, PreparedExternalPackageBundle,
};

#[derive(Debug, Clone)]
struct SourceEntry {
    source_path: String,
    segments: Vec<String>,
}

#[derive(Debug)]
struct PreparedExternalPackageApply {
    analysis: ExternalPackageAnalysis,
    prepared_apply: PreparedBundleApply,
}

pub fn analyze_external_package(
    request: AnalyzeExternalPackageRequest,
) -> AppResult<ExternalPackageAnalysis> {
    let source_path = request.source_path;
    if !source_path.exists() {
        return Err(AppError::NotFound(format!(
            "external package source does not exist: {}",
            source_path.display()
        )));
    }

    let source_kind = detect_source_kind(&source_path)?;
    let source_entries = collect_source_entries(&source_path, source_kind)?;
    let (entries, warnings) = classify_source_entries(&source_entries);

    Ok(build_analysis(
        source_path,
        source_kind,
        source_entries.len(),
        entries,
        warnings,
    ))
}

pub fn create_external_package_bundle(
    request: CreateExternalPackageBundleRequest,
) -> AppResult<PreparedExternalPackageBundle> {
    let (analysis, manifest) = prepare_external_package_artifacts(&request)?;

    let stage_dir = tempdir()?;
    let staged_installation = create_staging_installation(
        stage_dir.path(),
        request.source_flavor,
        request
            .source_platform
            .unwrap_or_else(HostPlatform::current),
    )?;
    materialize_analysis_to_installation(&analysis, &staged_installation)?;

    let output_path = request
        .output_path
        .clone()
        .or_else(|| Some(stage_dir.path().join("external-package.bundle.zip")));
    let bundle = pack_bundle(PackBundleRequest {
        installation: staged_installation,
        manifest: manifest.clone(),
        output_path,
        manifest_base_dir: None,
    })?;

    Ok(PreparedExternalPackageBundle {
        analysis,
        manifest,
        bundle,
        _stage_dir: stage_dir,
    })
}

pub fn plan_external_package_apply(
    request: PlanExternalPackageApplyRequest,
) -> AppResult<ExternalPackageApplyPlan> {
    let (analysis, manifest) = prepare_external_package_artifacts(&request.external_package)?;
    let entry_source_map = build_external_package_entry_source_map(&analysis)?;
    let source_path = analysis.source_path.clone();
    let source = PreparedApplySource::ExternalPackage {
        source_path: source_path.clone(),
        source_kind: analysis.source_kind,
        entry_source_map,
    };
    let plan = super::planner::plan_apply_from_source(
        &source_path,
        &request.installation,
        manifest,
        &request.apply_mappings,
        &source,
    )?;

    Ok(project_external_package_plan(analysis, plan))
}

fn prepare_external_package_artifacts(
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

fn prepare_external_package_apply(
    external_package: CreateExternalPackageBundleRequest,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<PreparedExternalPackageApply> {
    let (analysis, manifest) = prepare_external_package_artifacts(&external_package)?;
    let entry_source_map = build_external_package_entry_source_map(&analysis)?;
    let source_path = analysis.source_path.clone();
    let prepared_apply = super::planner::prepare_apply_from_source(
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
