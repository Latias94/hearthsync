use std::sync::atomic::{AtomicU64, Ordering};

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
    AddonIndexAttach,
    AddonIndexInstall,
    AddonIndexUpdate,
    AddonIndexRelink,
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
            Self::AddonIndexAttach => "addon_index_attach",
            Self::AddonIndexInstall => "addon_index_install",
            Self::AddonIndexUpdate => "addon_index_update",
            Self::AddonIndexRelink => "addon_index_relink",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskProgressCode {
    Preparing,
    Planning,
    BackingUp,
    Executing,
    Verifying,
    Completed,
    DownloadArchive,
    RemoveAddonDirectory,
    WriteAddonDirectory,
    ApplyMetadata,
    ClearRestoreGroup,
    RestoreEntry,
    ApplyOperation,
}

impl TaskProgressCode {
    pub fn for_phase(phase: TaskPhase) -> Self {
        match phase {
            TaskPhase::Preparing => Self::Preparing,
            TaskPhase::Planning => Self::Planning,
            TaskPhase::BackingUp => Self::BackingUp,
            TaskPhase::Executing => Self::Executing,
            TaskPhase::Verifying => Self::Verifying,
            TaskPhase::Completed => Self::Completed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskProgressEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub task: TaskKind,
    pub phase: TaskPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<TaskProgressCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_current: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_per_second: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskByteProgress {
    pub code: TaskProgressCode,
    pub bytes_current: u64,
    pub bytes_total: Option<u64>,
    pub bytes_per_second: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TaskProgressPayload {
    code: Option<TaskProgressCode>,
    current: Option<usize>,
    total: Option<usize>,
    bytes_current: Option<u64>,
    bytes_total: Option<u64>,
    bytes_per_second: Option<u64>,
}

impl TaskProgressPayload {
    fn phase(phase: TaskPhase) -> Self {
        Self {
            code: Some(TaskProgressCode::for_phase(phase)),
            ..Self::default()
        }
    }

    fn step(code: TaskProgressCode, current: usize, total: usize) -> Self {
        Self {
            code: Some(code),
            current: Some(current),
            total: Some(total),
            ..Self::default()
        }
    }

    fn byte(progress: TaskByteProgress) -> Self {
        Self {
            code: Some(progress.code),
            bytes_current: Some(progress.bytes_current),
            bytes_total: progress.bytes_total,
            bytes_per_second: progress.bytes_per_second,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskRun<T> {
    pub task_id: String,
    pub result: T,
    pub progress: Vec<TaskProgressEvent>,
}

pub trait CancellationToken {
    fn is_cancelled(&self) -> bool;
}

pub trait TaskProgressSink {
    fn push(&mut self, event: TaskProgressEvent);

    fn task_id(&self) -> Option<&str> {
        None
    }
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
    task_id: Option<String>,
    events: Vec<TaskProgressEvent>,
}

impl VecTaskProgressSink {
    pub fn with_task_id(task_id: impl Into<String>) -> Self {
        Self {
            task_id: Some(task_id.into()),
            events: Vec::new(),
        }
    }

    pub fn events(&self) -> &[TaskProgressEvent] {
        &self.events
    }

    pub fn into_events(self) -> Vec<TaskProgressEvent> {
        self.events
    }
}

impl TaskProgressSink for VecTaskProgressSink {
    fn push(&mut self, mut event: TaskProgressEvent) {
        attach_task_id_if_missing(&mut event, self.task_id());
        self.events.push(event);
    }

    fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
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
    task_id: Option<String>,
    on_progress: F,
}

impl<F> CallbackProgressSink<F> {
    pub fn new(on_progress: F) -> Self {
        Self {
            task_id: None,
            on_progress,
        }
    }

    pub fn with_task_id(task_id: impl Into<String>, on_progress: F) -> Self {
        Self {
            task_id: Some(task_id.into()),
            on_progress,
        }
    }
}

impl<F> TaskProgressSink for CallbackProgressSink<F>
where
    F: FnMut(TaskProgressEvent),
{
    fn push(&mut self, mut event: TaskProgressEvent) {
        attach_task_id_if_missing(&mut event, self.task_id());
        (self.on_progress)(event);
    }

    fn task_id(&self) -> Option<&str> {
        self.task_id.as_deref()
    }
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

pub fn emit_task_progress(
    sink: &mut impl TaskProgressSink,
    task: TaskKind,
    phase: TaskPhase,
    message: impl Into<String>,
) {
    push_task_progress_event(
        sink,
        task,
        phase,
        TaskProgressPayload::phase(phase),
        message,
    );
}

pub fn emit_task_step_progress(
    sink: &mut impl TaskProgressSink,
    task: TaskKind,
    phase: TaskPhase,
    code: TaskProgressCode,
    current: usize,
    total: usize,
    message: impl Into<String>,
) {
    push_task_progress_event(
        sink,
        task,
        phase,
        TaskProgressPayload::step(code, current, total),
        message,
    );
}

pub fn emit_task_byte_progress(
    sink: &mut impl TaskProgressSink,
    task: TaskKind,
    phase: TaskPhase,
    byte_progress: TaskByteProgress,
    message: impl Into<String>,
) {
    push_task_progress_event(
        sink,
        task,
        phase,
        TaskProgressPayload::byte(byte_progress),
        message,
    );
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

static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

fn next_task_id() -> String {
    format!("task-{:016x}", NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed))
}

fn attach_task_id_if_missing(event: &mut TaskProgressEvent, task_id: Option<&str>) {
    if event.task_id.is_none() {
        event.task_id = task_id.map(ToOwned::to_owned);
    }
}

fn push_task_progress_event(
    sink: &mut impl TaskProgressSink,
    task: TaskKind,
    phase: TaskPhase,
    payload: TaskProgressPayload,
    message: impl Into<String>,
) {
    sink.push(TaskProgressEvent {
        task_id: None,
        task,
        phase,
        code: payload.code,
        current: payload.current,
        total: payload.total,
        bytes_current: payload.bytes_current,
        bytes_total: payload.bytes_total,
        bytes_per_second: payload.bytes_per_second,
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::{
        AppError, TaskByteProgress, TaskKind, TaskPhase, TaskProgressCode, emit_task_byte_progress,
        emit_task_progress, emit_task_step_progress, ensure_task_not_cancelled,
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

        assert!(run.task_id.starts_with("task-"));
        assert_eq!(run.result, 42);
        assert_eq!(run.progress.len(), 1);
        assert_eq!(run.progress[0].task, TaskKind::ExternalPackageAnalyze);
        assert_eq!(
            run.progress[0].task_id.as_deref(),
            Some(run.task_id.as_str())
        );
        assert_eq!(run.progress[0].code, Some(TaskProgressCode::Preparing));
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
        assert!(
            seen.borrow()[0]
                .task_id
                .as_deref()
                .is_some_and(|task_id| task_id.starts_with("task-"))
        );
    }

    #[test]
    fn emit_task_step_progress_records_structured_step_fields() {
        let run = run_task_with_collected_progress(|_cancellation, progress| {
            emit_task_step_progress(
                progress,
                TaskKind::AddonInstall,
                TaskPhase::Executing,
                TaskProgressCode::WriteAddonDirectory,
                2,
                5,
                "Writing addon directory 2/5 `WeakAuras`",
            );
            Ok::<_, AppError>(())
        })
        .expect("run task");

        assert_eq!(run.progress.len(), 1);
        assert_eq!(
            run.progress[0].code,
            Some(TaskProgressCode::WriteAddonDirectory)
        );
        assert_eq!(run.progress[0].current, Some(2));
        assert_eq!(run.progress[0].total, Some(5));
        assert_eq!(
            run.progress[0].task_id.as_deref(),
            Some(run.task_id.as_str())
        );
    }

    #[test]
    fn emit_task_byte_progress_records_structured_byte_fields() {
        let run = run_task_with_collected_progress(|_cancellation, progress| {
            emit_task_byte_progress(
                progress,
                TaskKind::AddonUpdate,
                TaskPhase::Executing,
                TaskByteProgress {
                    code: TaskProgressCode::Executing,
                    bytes_current: 512,
                    bytes_total: Some(1024),
                    bytes_per_second: Some(256),
                },
                "Downloading addon archive",
            );
            Ok::<_, AppError>(())
        })
        .expect("run task");

        assert_eq!(run.progress.len(), 1);
        assert_eq!(run.progress[0].bytes_current, Some(512));
        assert_eq!(run.progress[0].bytes_total, Some(1024));
        assert_eq!(run.progress[0].bytes_per_second, Some(256));
    }
}
