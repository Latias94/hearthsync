use crate::core::error::AppResult;
use crate::core::task::{
    CancellationToken, TaskKind, TaskPhase, TaskProgressCode, TaskProgressSink,
    emit_task_step_progress, ensure_task_not_cancelled,
};

#[derive(Clone, Copy)]
pub(super) enum MutationProgressMode {
    Install,
    Update,
    Remove,
}

#[derive(Clone, Copy)]
pub(super) enum AddonMutationStep<'a> {
    RemoveAddonDirectory {
        addon_name: &'a str,
        current: usize,
        total: usize,
    },
    WriteAddonDirectory {
        addon_name: &'a str,
        current: usize,
        total: usize,
    },
}

pub(super) trait AddonMutationObserver {
    fn before_step(&mut self, _step: AddonMutationStep<'_>) -> AppResult<()> {
        Ok(())
    }
}

pub(super) struct TaskAddonMutationObserver<'a, TCancel, TProgress> {
    task: TaskKind,
    mode: MutationProgressMode,
    cancellation: &'a TCancel,
    progress: &'a mut TProgress,
}

impl<'a, TCancel, TProgress> TaskAddonMutationObserver<'a, TCancel, TProgress> {
    pub(super) fn new(
        task: TaskKind,
        mode: MutationProgressMode,
        cancellation: &'a TCancel,
        progress: &'a mut TProgress,
    ) -> Self {
        Self {
            task,
            mode,
            cancellation,
            progress,
        }
    }
}

impl<TCancel, TProgress> AddonMutationObserver for TaskAddonMutationObserver<'_, TCancel, TProgress>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    fn before_step(&mut self, step: AddonMutationStep<'_>) -> AppResult<()> {
        ensure_task_not_cancelled(self.cancellation, self.task, TaskPhase::Executing)?;
        let (code, current, total) = addon_mutation_step_progress(self.mode, step);
        emit_task_step_progress(
            self.progress,
            self.task,
            TaskPhase::Executing,
            code,
            current,
            total,
            addon_mutation_step_message(self.mode, step),
        );
        Ok(())
    }
}

fn addon_mutation_step_message(mode: MutationProgressMode, step: AddonMutationStep<'_>) -> String {
    match (mode, step) {
        (
            MutationProgressMode::Install,
            AddonMutationStep::WriteAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Installing addon directory {current}/{total} `{addon_name}`"),
        (
            MutationProgressMode::Update,
            AddonMutationStep::RemoveAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Removing existing addon directory {current}/{total} `{addon_name}`"),
        (
            MutationProgressMode::Update,
            AddonMutationStep::WriteAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Writing updated addon directory {current}/{total} `{addon_name}`"),
        (
            MutationProgressMode::Remove,
            AddonMutationStep::RemoveAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Removing addon directory {current}/{total} `{addon_name}`"),
        (MutationProgressMode::Install, AddonMutationStep::RemoveAddonDirectory { .. })
        | (MutationProgressMode::Remove, AddonMutationStep::WriteAddonDirectory { .. }) => {
            "Applying addon mutation".to_string()
        }
    }
}

fn addon_mutation_step_progress(
    mode: MutationProgressMode,
    step: AddonMutationStep<'_>,
) -> (TaskProgressCode, usize, usize) {
    match (mode, step) {
        (
            MutationProgressMode::Install,
            AddonMutationStep::WriteAddonDirectory { current, total, .. },
        )
        | (
            MutationProgressMode::Update,
            AddonMutationStep::WriteAddonDirectory { current, total, .. },
        ) => (TaskProgressCode::WriteAddonDirectory, current, total),
        (
            MutationProgressMode::Update,
            AddonMutationStep::RemoveAddonDirectory { current, total, .. },
        )
        | (
            MutationProgressMode::Remove,
            AddonMutationStep::RemoveAddonDirectory { current, total, .. },
        ) => (TaskProgressCode::RemoveAddonDirectory, current, total),
        (MutationProgressMode::Install, AddonMutationStep::RemoveAddonDirectory { .. })
        | (MutationProgressMode::Remove, AddonMutationStep::WriteAddonDirectory { .. }) => {
            (TaskProgressCode::Executing, 1, 1)
        }
    }
}
