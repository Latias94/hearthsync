use crate::core::error::AppResult;
use crate::core::task::{
    CallbackCancellationToken, CallbackProgressSink, NeverCancel, NoopProgressSink,
    TaskProgressEvent, TaskRun, VecTaskProgressSink, run_task_with_callbacks,
    run_task_with_collected_progress,
};

pub(super) fn run_direct_task<TResult, FTask>(task: FTask) -> AppResult<TResult>
where
    FTask: FnOnce(&NeverCancel, &mut NoopProgressSink) -> AppResult<TResult>,
{
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    task(&cancellation, &mut progress)
}

pub(super) fn run_collecting_task<TResult, FTask>(task: FTask) -> AppResult<TaskRun<TResult>>
where
    FTask: FnOnce(&NeverCancel, &mut VecTaskProgressSink) -> AppResult<TResult>,
{
    run_task_with_collected_progress(task)
}

pub(super) fn run_callback_task<TResult, FTask, FCancel, FProgress>(
    is_cancelled: FCancel,
    on_progress: FProgress,
    task: FTask,
) -> AppResult<TResult>
where
    FTask: FnOnce(
        &CallbackCancellationToken<FCancel>,
        &mut CallbackProgressSink<FProgress>,
    ) -> AppResult<TResult>,
    FCancel: Fn() -> bool,
    FProgress: FnMut(TaskProgressEvent),
{
    run_task_with_callbacks(is_cancelled, on_progress, task)
}
