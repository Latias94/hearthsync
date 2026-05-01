use super::event::{TaskByteProgress, TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent};
use super::sink::TaskProgressSink;

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
