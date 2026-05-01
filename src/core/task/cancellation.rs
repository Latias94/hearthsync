use crate::core::error::{AppError, AppResult};

use super::event::{TaskKind, TaskPhase};

pub trait CancellationToken {
    fn is_cancelled(&self) -> bool;
}

#[derive(Debug, Default)]
pub struct NeverCancel;

impl CancellationToken for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

pub struct CallbackCancellationToken<F> {
    is_cancelled: F,
}

impl<F> CallbackCancellationToken<F> {
    pub fn new(is_cancelled: F) -> Self {
        Self { is_cancelled }
    }
}

impl<F> CancellationToken for CallbackCancellationToken<F>
where
    F: Fn() -> bool,
{
    fn is_cancelled(&self) -> bool {
        (self.is_cancelled)()
    }
}

pub fn ensure_task_not_cancelled(
    token: &impl CancellationToken,
    task: TaskKind,
    phase: TaskPhase,
) -> AppResult<()> {
    if token.is_cancelled() {
        Err(AppError::Cancelled(format!(
            "{} cancelled during {}",
            task.as_str(),
            phase.as_str()
        )))
    } else {
        Ok(())
    }
}
