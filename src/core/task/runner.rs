use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::core::error::AppResult;

use super::cancellation::{CallbackCancellationToken, NeverCancel};
use super::event::TaskProgressEvent;
use super::sink::{CallbackProgressSink, VecTaskProgressSink};

#[derive(Debug, Clone, Serialize)]
pub struct TaskRun<T> {
    pub task_id: String,
    pub result: T,
    pub progress: Vec<TaskProgressEvent>,
}

pub fn run_task_with_collected_progress<TResult, FTask>(task: FTask) -> AppResult<TaskRun<TResult>>
where
    FTask: FnOnce(&NeverCancel, &mut VecTaskProgressSink) -> AppResult<TResult>,
{
    let task_id = next_task_id();
    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::with_task_id(task_id.clone());
    let result = task(&cancellation, &mut progress)?;

    Ok(TaskRun {
        task_id,
        result,
        progress: progress.into_events(),
    })
}

pub fn run_task_with_callbacks<TResult, FTask, FCancel, FProgress>(
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
    let task_id = next_task_id();
    let cancellation = CallbackCancellationToken::new(is_cancelled);
    let mut progress = CallbackProgressSink::with_task_id(task_id, on_progress);
    task(&cancellation, &mut progress)
}

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

fn next_task_id() -> String {
    format!("task-{:016x}", NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
}
