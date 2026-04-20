use std::path::PathBuf;

use super::super::apply_model::prepared::{PreparedApplyOperation, PreparedApplySource};
use super::super::execution::apply::execute_apply_operations;
use super::super::execution::rollback::rollback_or_report_apply_error;
use super::super::types::BundleApplyPlan;
use super::task_context::BundleApplyTaskContext;
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::manifest::BundleManifest;
use crate::core::task::{
    CancellationToken, TaskPhase, TaskProgressSink, emit_task_progress, ensure_task_not_cancelled,
};

pub(super) struct BundleExecution {
    pub(super) backup_path: Option<PathBuf>,
    pub(super) written_files: usize,
    pub(super) rewritten_files: usize,
}

pub(super) struct BundleExecutor<'a> {
    installation: &'a DetectedFlavorInstallation,
    backup_output_path: Option<PathBuf>,
    task_context: BundleApplyTaskContext,
}

impl<'a> BundleExecutor<'a> {
    pub(super) fn new(
        installation: &'a DetectedFlavorInstallation,
        backup_output_path: Option<PathBuf>,
        task_context: BundleApplyTaskContext,
    ) -> Self {
        Self {
            installation,
            backup_output_path,
            task_context,
        }
    }

    pub(super) fn execute<TCancel, TProgress>(
        &self,
        source: &PreparedApplySource,
        plan: &BundleApplyPlan,
        execution_operations: &[PreparedApplyOperation],
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<BundleExecution>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        if plan.manifest.apply.create_backup
            && !backup_groups_for_manifest(&plan.manifest).is_empty()
        {
            emit_task_progress(
                progress,
                self.task_context.task_kind(),
                TaskPhase::BackingUp,
                self.task_context.backup_message(),
            );
            ensure_task_not_cancelled(
                cancellation,
                self.task_context.task_kind(),
                TaskPhase::BackingUp,
            )?;
        }
        let backup_path = self.create_backup(plan)?;
        emit_task_progress(
            progress,
            self.task_context.task_kind(),
            TaskPhase::Executing,
            self.task_context
                .executing_message(execution_operations.len()),
        );
        ensure_task_not_cancelled(
            cancellation,
            self.task_context.task_kind(),
            TaskPhase::Executing,
        )?;

        match execute_apply_operations(
            source,
            execution_operations,
            &plan.manifest,
            |operation_index, operation_count, operation| {
                emit_task_progress(
                    progress,
                    self.task_context.task_kind(),
                    TaskPhase::Executing,
                    self.task_context.operation_message(
                        operation_index,
                        operation_count,
                        operation,
                    ),
                );
                ensure_task_not_cancelled(
                    cancellation,
                    self.task_context.task_kind(),
                    TaskPhase::Executing,
                )
            },
        ) {
            Ok((written_files, rewritten_files)) => Ok(BundleExecution {
                backup_path,
                written_files,
                rewritten_files,
            }),
            Err(error) => rollback_or_report_apply_error(
                error,
                backup_path.as_deref(),
                self.installation,
                self.task_context.failure_label(),
            ),
        }
    }

    fn create_backup(&self, plan: &BundleApplyPlan) -> AppResult<Option<PathBuf>> {
        if !plan.manifest.apply.create_backup {
            return Ok(None);
        }

        let groups = backup_groups_for_manifest(&plan.manifest);
        if groups.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                create_backup(BackupRequest {
                    installation: self.installation.clone(),
                    output_path: self.backup_output_path.clone(),
                    groups,
                    label: Some(self.task_context.backup_label().to_string()),
                })?
                .archive_path,
            ))
        }
    }
}

fn backup_groups_for_manifest(manifest: &BundleManifest) -> Vec<BackupGroup> {
    let mut groups = Vec::new();

    if !manifest.resources.addons.is_empty()
        || manifest.resources.addon_lock
        || !manifest.resources.addon_indexes.is_empty()
    {
        groups.push(BackupGroup::Addons);
    }
    if manifest.resources.wtf_common || !manifest.resources.wtf_characters.is_empty() {
        groups.push(BackupGroup::Wtf);
    }
    if manifest.resources.fonts {
        groups.push(BackupGroup::Fonts);
    }
    if !manifest.resources.interface_assets.is_empty() {
        groups.push(BackupGroup::InterfaceAssets);
    }

    groups
}
