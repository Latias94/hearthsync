use super::planner::prepare_bundle_apply;
use super::*;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};

struct BundleExecution {
    backup_path: Option<PathBuf>,
    written_files: usize,
    rewritten_files: usize,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum BundleApplyTaskContext {
    BundleApply,
    ExternalPackageApply,
}

impl BundleApplyTaskContext {
    fn task_kind(self) -> TaskKind {
        match self {
            Self::BundleApply => TaskKind::BundleApply,
            Self::ExternalPackageApply => TaskKind::ExternalPackageApply,
        }
    }

    fn planning_message(self, operation_count: usize) -> String {
        match self {
            Self::BundleApply => {
                format!("Prepared bundle apply plan with {operation_count} operation(s)")
            }
            Self::ExternalPackageApply => {
                format!("Prepared external package apply plan with {operation_count} operation(s)")
            }
        }
    }

    fn dry_run_completed_message(self) -> &'static str {
        match self {
            Self::BundleApply => "Bundle dry run completed without filesystem writes",
            Self::ExternalPackageApply => {
                "External package dry run completed without filesystem writes"
            }
        }
    }

    fn backup_message(self) -> &'static str {
        match self {
            Self::BundleApply => "Creating backup checkpoint before bundle apply",
            Self::ExternalPackageApply => {
                "Creating backup checkpoint before external package apply"
            }
        }
    }

    fn executing_message(self, operation_count: usize) -> String {
        match self {
            Self::BundleApply => {
                format!("Executing {operation_count} planned bundle operation(s)")
            }
            Self::ExternalPackageApply => {
                format!("Executing {operation_count} planned external package operation(s)")
            }
        }
    }

    fn completed_message(self, written_files: usize) -> String {
        match self {
            Self::BundleApply => {
                format!("Bundle apply completed with {written_files} written file(s)")
            }
            Self::ExternalPackageApply => {
                format!("External package apply completed with {written_files} written file(s)")
            }
        }
    }
}

struct BundleExecutor<'a> {
    installation: &'a DetectedFlavorInstallation,
    backup_output_path: Option<PathBuf>,
    task_context: BundleApplyTaskContext,
}

pub fn unpack_bundle(request: UnpackBundleRequest) -> AppResult<UnpackedBundle> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    unpack_bundle_task(request, &cancellation, &mut progress)
}

pub fn unpack_bundle_task<TCancel, TProgress>(
    request: UnpackBundleRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UnpackedBundle>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let task_context = BundleApplyTaskContext::BundleApply;
    emit_task_progress(
        progress,
        task_context.task_kind(),
        TaskPhase::Preparing,
        format!(
            "Inspecting bundle `{}` for target `{}`",
            request.bundle_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, task_context.task_kind(), TaskPhase::Preparing)?;

    let prepared = prepare_bundle_apply(
        &request.bundle_path,
        &request.installation,
        &request.apply_mappings,
    )?;

    execute_prepared_apply_with_context(
        prepared,
        request.installation,
        request.dry_run,
        request.backup_output_path,
        cancellation,
        progress,
        task_context,
    )
}

pub(super) fn execute_prepared_apply_with_context<TCancel, TProgress>(
    prepared: PreparedBundleApply,
    installation: DetectedFlavorInstallation,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    cancellation: &TCancel,
    progress: &mut TProgress,
    task_context: BundleApplyTaskContext,
) -> AppResult<UnpackedBundle>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let PreparedBundleApply {
        source,
        plan,
        execution_operations,
    } = prepared;
    emit_task_progress(
        progress,
        task_context.task_kind(),
        TaskPhase::Planning,
        task_context.planning_message(plan.operations.len()),
    );
    ensure_task_not_cancelled(cancellation, task_context.task_kind(), TaskPhase::Planning)?;

    if dry_run {
        let result = UnpackedBundle {
            bundle_path: plan.bundle_path,
            target_flavor_root: plan.target_flavor_root,
            dry_run: true,
            planned_files: plan.operations.len(),
            written_files: 0,
            rewritten_files: 0,
            backup_path: None,
            selected_target_accounts: plan.selected_target_accounts,
            plan_summary: plan.summary,
            character_mappings: plan.character_mappings,
            manifest: plan.manifest,
        };
        emit_task_progress(
            progress,
            task_context.task_kind(),
            TaskPhase::Completed,
            task_context.dry_run_completed_message(),
        );
        return Ok(result);
    }

    let execution = BundleExecutor {
        installation: &installation,
        backup_output_path,
        task_context,
    }
    .execute(
        &source,
        &plan,
        &execution_operations,
        cancellation,
        progress,
    )?;

    let result = UnpackedBundle {
        bundle_path: plan.bundle_path,
        target_flavor_root: plan.target_flavor_root,
        dry_run: false,
        planned_files: plan.operations.len(),
        written_files: execution.written_files,
        rewritten_files: execution.rewritten_files,
        backup_path: execution.backup_path,
        selected_target_accounts: plan.selected_target_accounts,
        plan_summary: plan.summary,
        character_mappings: plan.character_mappings,
        manifest: plan.manifest,
    };
    emit_task_progress(
        progress,
        task_context.task_kind(),
        TaskPhase::Completed,
        task_context.completed_message(result.written_files),
    );
    Ok(result)
}

impl<'a> BundleExecutor<'a> {
    fn execute<TCancel, TProgress>(
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

        match execute_apply_operations(source, execution_operations, &plan.manifest) {
            Ok((written_files, rewritten_files)) => Ok(BundleExecution {
                backup_path,
                written_files,
                rewritten_files,
            }),
            Err(error) => {
                rollback_or_report_apply_error(error, backup_path.as_deref(), self.installation)
            }
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
                    label: Some("bundle-apply".to_string()),
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
