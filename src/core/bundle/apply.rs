use super::planner::prepare_bundle_apply;
use super::*;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};

mod executor;
mod result;
mod task_context;

use executor::BundleExecutor;
use result::{project_dry_run_result, project_executed_result};
pub(crate) use task_context::BundleApplyTaskContext;

pub fn unpack_bundle(request: UnpackBundleRequest) -> AppResult<UnpackedBundle> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    unpack_bundle_task(request, &cancellation, &mut progress)
}

pub fn unpack_bundle_task<TCancel, TProgress>(
    request: UnpackBundleRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UnpackedBundle>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let task_context = BundleApplyTaskContext::BundleApply;
    emit_task_progress(
        progress,
        task_context.task_kind(),
        TaskPhase::Preparing,
        format!(
            "Inspecting bundle `{}` for target `{}`",
            request.bundle_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, task_context.task_kind(), TaskPhase::Preparing)?;

    let prepared = prepare_bundle_apply(
        &request.bundle_path,
        &request.installation,
        &request.apply_mappings,
    )?;

    execute_prepared_apply_with_context(
        prepared,
        request.installation,
        request.dry_run,
        request.backup_output_path,
        cancellation,
        progress,
        task_context,
    )
}

pub(super) fn execute_prepared_apply_with_context<TCancel, TProgress>(
    prepared: PreparedBundleApply,
    installation: DetectedFlavorInstallation,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    cancellation: &TCancel,
    progress: &mut TProgress,
    task_context: BundleApplyTaskContext,
) -> AppResult<UnpackedBundle>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let PreparedBundleApply {
        source,
        plan,
        execution_operations,
    } = prepared;
    emit_task_progress(
        progress,
        task_context.task_kind(),
        TaskPhase::Planning,
        task_context.planning_message(plan.operations.len()),
    );
    ensure_task_not_cancelled(cancellation, task_context.task_kind(), TaskPhase::Planning)?;

    if dry_run {
        let result = project_dry_run_result(plan);
        emit_task_progress(
            progress,
            task_context.task_kind(),
            TaskPhase::Completed,
            task_context.dry_run_completed_message(),
        );
        return Ok(result);
    }

    let execution = BundleExecutor::new(&installation, backup_output_path, task_context).execute(
        &source,
        &plan,
        &execution_operations,
        cancellation,
        progress,
    )?;

    let result = project_executed_result(plan, execution);
    emit_task_progress(
        progress,
        task_context.task_kind(),
        TaskPhase::Completed,
        task_context.completed_message(result.written_files),
    );
    Ok(result)
}
