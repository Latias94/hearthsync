use serde::Serialize;

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    BackupRestore,
    BundleApply,
    AddonLockApply,
    AddonInstall,
    AddonUpdate,
    AddonRemove,
    AddonIndexInstall,
    AddonIndexUpdate,
    ExternalPackageAnalyze,
    ExternalPackagePlan,
    ExternalPackageApply,
}

impl TaskKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BackupRestore => "backup_restore",
            Self::BundleApply => "bundle_apply",
            Self::AddonLockApply => "addon_lock_apply",
            Self::AddonInstall => "addon_install",
            Self::AddonUpdate => "addon_update",
            Self::AddonRemove => "addon_remove",
            Self::AddonIndexInstall => "addon_index_install",
            Self::AddonIndexUpdate => "addon_index_update",
            Self::ExternalPackageAnalyze => "external_package_analyze",
            Self::ExternalPackagePlan => "external_package_plan",
            Self::ExternalPackageApply => "external_package_apply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPhase {
    Preparing,
    Planning,
    BackingUp,
    Executing,
    Verifying,
    Completed,
}

impl TaskPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Planning => "planning",
            Self::BackingUp => "backing_up",
            Self::Executing => "executing",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskProgressEvent {
    pub task: TaskKind,
    pub phase: TaskPhase,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRun<T> {
    pub result: T,
    pub progress: Vec<TaskProgressEvent>,
}

pub trait CancellationToken {
    fn is_cancelled(&self) -> bool;
}

pub trait TaskProgressSink {
    fn push(&mut self, event: TaskProgressEvent);
}

#[derive(Debug, Default)]
pub struct NeverCancel;

impl CancellationToken for NeverCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
}

#[derive(Debug, Default)]
pub struct NoopProgressSink;

impl TaskProgressSink for NoopProgressSink {
    fn push(&mut self, _event: TaskProgressEvent) {}
}

#[derive(Debug, Default)]
pub struct VecTaskProgressSink {
    events: Vec<TaskProgressEvent>,
}

impl VecTaskProgressSink {
    pub fn events(&self) -> &[TaskProgressEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<TaskProgressEvent> {
        self.events
    }
}

impl TaskProgressSink for VecTaskProgressSink {
    fn push(&mut self, event: TaskProgressEvent) {
        self.events.push(event);
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

pub struct CallbackProgressSink<F> {
    on_progress: F,
}

impl<F> CallbackProgressSink<F> {
    pub fn new(on_progress: F) -> Self {
        Self { on_progress }
    }
}

impl<F> TaskProgressSink for CallbackProgressSink<F>
where
    F: FnMut(TaskProgressEvent),
{
    fn push(&mut self, event: TaskProgressEvent) {
        (self.on_progress)(event);
    }
}

pub fn run_task_with_collected_progress<TResult, FTask>(task: FTask) -> AppResult<TaskRun<TResult>>
where
    FTask: FnOnce(&NeverCancel, &mut VecTaskProgressSink) -> AppResult<TResult>,
{
    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = task(&cancellation, &mut progress)?;

    Ok(TaskRun {
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
    let cancellation = CallbackCancellationToken::new(is_cancelled);
    let mut progress = CallbackProgressSink::new(on_progress);
    task(&cancellation, &mut progress)
}

pub fn emit_task_progress(
    sink: &mut impl TaskProgressSink,
    task: TaskKind,
    phase: TaskPhase,
    message: impl Into<String>,
) {
    sink.push(TaskProgressEvent {
        task,
        phase,
        message: message.into(),
    });
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

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::{
        AppError, TaskKind, TaskPhase, emit_task_progress, ensure_task_not_cancelled,
        run_task_with_callbacks, run_task_with_collected_progress,
    };

    #[test]
    fn run_task_with_collected_progress_returns_result_and_events() {
        let run = run_task_with_collected_progress(|cancellation, progress| {
            ensure_task_not_cancelled(
                cancellation,
                TaskKind::ExternalPackageAnalyze,
                TaskPhase::Preparing,
            )?;
            emit_task_progress(
                progress,
                TaskKind::ExternalPackageAnalyze,
                TaskPhase::Preparing,
                "collect progress",
            );
            Ok::<_, AppError>(42usize)
        })
        .expect("run task");

        assert_eq!(run.result, 42);
        assert_eq!(run.progress.len(), 1);
        assert_eq!(run.progress[0].task, TaskKind::ExternalPackageAnalyze);
    }

    #[test]
    fn run_task_with_callbacks_forwards_progress_and_cancellation() {
        let seen = RefCell::new(Vec::new());
        let cancellation_checks = Cell::new(0usize);

        let error = run_task_with_callbacks(
            || {
                let next = cancellation_checks.get() + 1;
                cancellation_checks.set(next);
                next >= 2
            },
            |event| seen.borrow_mut().push(event),
            |cancellation, progress| {
                emit_task_progress(
                    progress,
                    TaskKind::ExternalPackagePlan,
                    TaskPhase::Preparing,
                    "callback progress",
                );
                ensure_task_not_cancelled(
                    cancellation,
                    TaskKind::ExternalPackagePlan,
                    TaskPhase::Preparing,
                )?;
                ensure_task_not_cancelled(
                    cancellation,
                    TaskKind::ExternalPackagePlan,
                    TaskPhase::Planning,
                )
            },
        )
        .expect_err("task should cancel");

        assert!(matches!(error, AppError::Cancelled(_)));
        assert_eq!(seen.borrow().len(), 1);
        assert_eq!(seen.borrow()[0].phase, TaskPhase::Preparing);
    }
}
