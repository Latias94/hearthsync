mod execution;
mod planning;
mod result;

use self::execution::{execute_index_attach_plan, index_attach_execute_message};
use self::planning::prepare_index_attach_with_provider;
use self::result::{index_attach_result, result_ready_for_attach};
use super::*;

pub fn attach_addons_from_index(
    request: AddonIndexAttachRequest,
) -> AppResult<AddonIndexAttachResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    attach_addons_from_index_task(request, &cancellation, &mut progress)
}

pub fn attach_addons_from_index_task<TCancel, TProgress>(
    request: AddonIndexAttachRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexAttachResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    attach_addons_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn attach_addons_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexAttachRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexAttachResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexAttach,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index attach from `{}` for `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexAttach,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_attach_with_provider(provider, request, cancellation, progress)?;
    if plan.dry_run {
        let result = index_attach_result(plan, false);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexAttach,
            TaskPhase::Completed,
            format!(
                "Addon index attach dry run completed with {} planned change(s) and {} blocking package(s)",
                result.change_package_count, result.blocked_package_count
            ),
        );
        return Ok(result);
    }

    if !result_ready_for_attach(&plan) && !plan.apply_ready_only {
        let result = index_attach_result(plan, false);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexAttach,
            TaskPhase::Completed,
            format!(
                "Addon index attach blocked by {} package(s); no registry changes were written",
                result.blocked_package_count
            ),
        );
        return Ok(result);
    }

    if plan.changes.is_empty() {
        let result = index_attach_result(plan, false);
        let message = if result.blocked_package_count > 0 && result.dry_run {
            "Addon index attach dry run found blocked packages and no ready registry changes"
        } else if result.blocked_package_count > 0 {
            "Addon index attach found blocked packages and no ready registry changes to apply"
        } else {
            "Addon index attach found no registry changes to apply"
        };
        emit_task_progress(
            progress,
            TaskKind::AddonIndexAttach,
            TaskPhase::Completed,
            message,
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonIndexAttach,
        TaskPhase::Executing,
        index_attach_execute_message(&plan),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexAttach,
        TaskPhase::Executing,
    )?;

    let result = execute_index_attach_plan(plan)?;
    emit_task_progress(
        progress,
        TaskKind::AddonIndexAttach,
        TaskPhase::Completed,
        format!(
            "Addon index attach completed with {} attached package(s)",
            result.attached_package_count
        ),
    );
    Ok(result)
}
