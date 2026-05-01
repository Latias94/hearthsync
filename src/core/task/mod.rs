mod cancellation;
mod event;
mod progress;
mod runner;
mod sink;

pub use cancellation::{
    CallbackCancellationToken, CancellationToken, NeverCancel, ensure_task_not_cancelled,
};
pub use event::{TaskByteProgress, TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent};
pub use progress::{emit_task_byte_progress, emit_task_progress, emit_task_step_progress};
pub use runner::{TaskRun, run_task_with_callbacks, run_task_with_collected_progress};
pub use sink::{CallbackProgressSink, NoopProgressSink, TaskProgressSink, VecTaskProgressSink};

#[cfg(test)]
mod tests;
