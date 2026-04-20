use std::path::PathBuf;

use super::super::apply_model::PreparedBundleApply;
use super::super::types::UnpackedBundle;
use super::BundleApplyTaskContext;
use super::executor::BundleExecutor;
use super::result::{project_dry_run_result, project_executed_result};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, TaskPhase, TaskProgressSink, emit_task_progress, ensure_task_not_cancelled,
};

pub(in crate::core::bundle) fn execute_prepared_apply_with_context<TCancel, TProgress>(
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
