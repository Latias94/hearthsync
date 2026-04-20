use super::planner::pipeline::prepare_bundle_apply;
use super::types::{UnpackBundleRequest, UnpackedBundle};
use crate::core::error::AppResult;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};
use task_context::BundleApplyTaskContext;

mod executor;
pub(super) mod pipeline;
mod result;
pub(super) mod task_context;

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

    pipeline::execute_prepared_apply_with_context(
        prepared,
        request.installation,
        request.dry_run,
        request.backup_output_path,
        cancellation,
        progress,
        task_context,
    )
}
