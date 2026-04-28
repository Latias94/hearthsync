use crate::core::error::AppResult;

use super::TaskProgressEvent;

pub struct AppLiveTask<FCancel, FProgress> {
    is_cancelled: FCancel,
    on_progress: FProgress,
}

impl<FCancel, FProgress> AppLiveTask<FCancel, FProgress> {
    pub fn new(is_cancelled: FCancel, on_progress: FProgress) -> Self {
        Self {
            is_cancelled,
            on_progress,
        }
    }

    pub(crate) fn into_callbacks(self) -> (FCancel, FProgress) {
        (self.is_cancelled, self.on_progress)
    }
}

pub(in crate::core::app) fn run_app_live_task<TResult, FCancel, FProgress, FInvoke>(
    live_task: AppLiveTask<FCancel, FProgress>,
    invoke: FInvoke,
) -> AppResult<TResult>
where
    FCancel: Fn() -> bool,
    FProgress: FnMut(TaskProgressEvent),
    FInvoke: FnOnce(FCancel, FProgress) -> AppResult<TResult>,
{
    let (is_cancelled, on_progress) = live_task.into_callbacks();
    invoke(is_cancelled, on_progress)
}
