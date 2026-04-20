use super::super::apply::{BundleApplyTaskContext, execute_prepared_apply_with_context};
use super::analyze::analyze_external_package;
use super::plan::plan_external_package_apply;
use super::prepare::prepare_external_package_apply;
use super::projection::project_applied_external_package;
use super::types::{
    AnalyzeExternalPackageRequest, AppliedExternalPackage, ApplyExternalPackageRequest,
    ExternalPackageAnalysis, ExternalPackageApplyPlan, PlanExternalPackageApplyRequest,
};
use crate::core::error::AppResult;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};

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
    let result = execute_prepared_apply_with_context(
        prepared.prepared_apply,
        request.installation,
        request.dry_run,
        request.backup_output_path,
        cancellation,
        progress,
        BundleApplyTaskContext::ExternalPackageApply,
    )?;

    Ok(project_applied_external_package(prepared.analysis, result))
}
