use std::cell::{Cell, RefCell};

use crate::core::error::AppError;

use super::{
    TaskByteProgress, TaskKind, TaskPhase, TaskProgressCode, emit_task_byte_progress,
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
