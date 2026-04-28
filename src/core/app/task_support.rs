use crate::core::error::AppResult;
use crate::core::task::{
    CallbackCancellationToken, CallbackProgressSink, NeverCancel, TaskProgressEvent, TaskRun,
    VecTaskProgressSink, run_task_with_callbacks, run_task_with_collected_progress,
};

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn run_direct_task<TResult, FTask>(task: FTask) -> AppResult<TResult>
where
    FTask: FnOnce(&NeverCancel, &mut crate::core::task::NoopProgressSink) -> AppResult<TResult>,
{
    let cancellation = NeverCancel;
    let mut progress = crate::core::task::NoopProgressSink;
    task(&cancellation, &mut progress)
}

pub(super) fn run_collecting_task<TResult, FTask>(task: FTask) -> AppResult<TaskRun<TResult>>
where
    FTask: FnOnce(&NeverCancel, &mut VecTaskProgressSink) -> AppResult<TResult>,
{
    run_task_with_collected_progress(task)
}

#[cfg_attr(not(test), allow(dead_code))]
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

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn run_service_task_direct<TService, TRequest, TResult, FTask>(
    service: &TService,
    request: TRequest,
    task: FTask,
) -> AppResult<TResult>
where
    FTask: FnOnce(
        &TService,
        TRequest,
        &NeverCancel,
        &mut crate::core::task::NoopProgressSink,
    ) -> AppResult<TResult>,
{
    run_direct_task(|cancellation, progress| task(service, request, cancellation, progress))
}

pub(super) fn run_service_task_collecting<TService, TRequest, TResult, FTask>(
    service: &TService,
    request: TRequest,
    task: FTask,
) -> AppResult<TaskRun<TResult>>
where
    FTask:
        FnOnce(&TService, TRequest, &NeverCancel, &mut VecTaskProgressSink) -> AppResult<TResult>,
{
    run_collecting_task(|cancellation, progress| task(service, request, cancellation, progress))
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn run_service_task_with_callbacks<
    TService,
    TRequest,
    TResult,
    FTask,
    FCancel,
    FProgress,
>(
    service: &TService,
    request: TRequest,
    is_cancelled: FCancel,
    on_progress: FProgress,
    task: FTask,
) -> AppResult<TResult>
where
    FTask: FnOnce(
        &TService,
        TRequest,
        &CallbackCancellationToken<FCancel>,
        &mut CallbackProgressSink<FProgress>,
    ) -> AppResult<TResult>,
    FCancel: Fn() -> bool,
    FProgress: FnMut(TaskProgressEvent),
{
    run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
        task(service, request, cancellation, progress)
    })
}
