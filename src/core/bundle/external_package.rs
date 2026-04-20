use tempfile::tempdir;

mod analysis;
mod classify;
mod manifest;
mod materialize;
mod normalized;
mod projection;
mod source;
mod types;

use super::*;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform};
use crate::core::manifest::BundleManifest;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};
use analysis::build_analysis;
use classify::classify_source_entries;
pub(crate) use manifest::author_package_apply_defaults;
use manifest::build_external_manifest;
use materialize::{create_staging_installation, materialize_analysis_to_installation};
use normalized::{build_external_package_entry_source_map, validate_unique_normalized_paths};
use projection::{project_applied_external_package, project_external_package_plan};
use source::{collect_source_entries, detect_source_kind};
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

pub fn analyze_external_package_task<TCancel, TProgress>(
    request: AnalyzeExternalPackageRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<ExternalPackageAnalysis>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Preparing,
        format!(
            "Inspecting external package source `{}`",
            request.source_path.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Preparing,
    )?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Planning,
        "Classifying external package resources and warnings",
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Planning,
    )?;

    let analysis = analyze_external_package(request)?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageAnalyze,
        TaskPhase::Completed,
        format!(
            "External package analysis completed with {} normalized file(s) and {} warning(s)",
            analysis.summary.normalized_files, analysis.summary.warning_count
        ),
    );
    Ok(analysis)
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

pub fn plan_external_package_apply_task<TCancel, TProgress>(
    request: PlanExternalPackageApplyRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<ExternalPackageApplyPlan>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Preparing,
        format!(
            "Normalizing external package `{}` for planning",
            request.external_package.source_path.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Preparing,
    )?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Planning,
        "Building apply plan for normalized external package",
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Planning,
    )?;

    let plan = plan_external_package_apply(request)?;
    emit_task_progress(
        progress,
        TaskKind::ExternalPackagePlan,
        TaskPhase::Completed,
        format!(
            "External package plan completed with {} operation(s)",
            plan.operations.len()
        ),
    );
    Ok(plan)
}

pub fn apply_external_package(
    request: ApplyExternalPackageRequest,
) -> AppResult<AppliedExternalPackage> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    apply_external_package_task(request, &cancellation, &mut progress)
}

pub fn apply_external_package_task<TCancel, TProgress>(
    request: ApplyExternalPackageRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AppliedExternalPackage>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::ExternalPackageApply,
        TaskPhase::Preparing,
        format!(
            "Normalizing external package `{}` for direct apply",
            request.external_package.source_path.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::ExternalPackageApply,
        TaskPhase::Preparing,
    )?;

    let prepared = prepare_external_package_apply(
        request.external_package,
        &request.installation,
        &request.apply_mappings,
    )?;
    let result = super::apply::execute_prepared_apply_with_context(
        prepared.prepared_apply,
        request.installation,
        request.dry_run,
        request.backup_output_path,
        cancellation,
        progress,
        super::apply::BundleApplyTaskContext::ExternalPackageApply,
    )?;

    Ok(project_applied_external_package(prepared.analysis, result))
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
