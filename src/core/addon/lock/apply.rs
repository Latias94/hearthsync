use super::apply_execute::execute_prepared_addon_lock_apply;
use super::apply_prepare::prepare_addon_lock_apply;
use super::plan::build_addon_lock_plan;
use super::source_resolution::resolved_source_override_map;
use super::verify::verify_addon_lock;
use super::*;

pub fn apply_addon_lock_sync(request: AddonLockApplyRequest) -> AppResult<AddonLockApplyResult> {
    let plan = build_addon_lock_plan(
        &request.installation,
        request.lock_path.as_deref(),
        &request.source_overrides,
    )?;
    let source_overrides =
        resolved_source_override_map(&plan.result.lock_path, &request.source_overrides)?;
    ensure_plan_is_applyable(&plan, request.replace_existing)?;

    let prepared = prepare_addon_lock_apply(&plan, &source_overrides, &request.installation)?;
    let backup_path = create_apply_backup(&request, prepared.is_empty())?;

    if let Err(error) =
        execute_prepared_addon_lock_apply(&request.installation, prepared, request.replace_existing)
    {
        return rollback_or_report_addon_error(
            error,
            backup_path.as_deref(),
            &request.installation,
        );
    }

    let verification = verify_addon_lock(&request.installation, Some(&plan.result.lock_path))?;
    Ok(AddonLockApplyResult {
        lock_path: plan.result.lock_path,
        installation_root: plan.result.installation_root,
        install_count: plan.result.install_count,
        update_count: plan.result.update_count,
        remove_count: plan.result.remove_count,
        metadata_only_count: plan.result.metadata_only_count,
        unchanged_count: plan.result.unchanged_count,
        blocked_count: plan.result.blocked_count,
        untracked_addons: verification.untracked_addons.clone(),
        actions: plan.result.actions,
        verification,
    })
}

fn ensure_plan_is_applyable(
    plan: &super::plan::AddonLockPlanContext,
    replace_existing: bool,
) -> AppResult<()> {
    let blocked_actions = plan
        .actions
        .iter()
        .filter(|action| !action.action.blocked_reasons.is_empty())
        .collect::<Vec<_>>();
    if !blocked_actions.is_empty() {
        return Err(AppError::Validation(format!(
            "cannot apply addon lock because some actions are blocked: {}",
            blocked_actions
                .iter()
                .map(|action| {
                    format!(
                        "{} ({})",
                        action.action.package_id,
                        action.action.blocked_reasons.join("; ")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let replace_required = plan
        .actions
        .iter()
        .filter(|action| {
            action.action.requires_replace_existing
                && matches!(
                    action.action.kind,
                    AddonLockSyncActionKind::Install | AddonLockSyncActionKind::Update
                )
        })
        .collect::<Vec<_>>();
    if !replace_existing && !replace_required.is_empty() {
        return Err(AppError::Validation(format!(
            "lock apply needs `--replace-existing` for packages: {}",
            replace_required
                .iter()
                .map(|action| action.action.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(())
}

fn create_apply_backup(
    request: &AddonLockApplyRequest,
    prepared_is_empty: bool,
) -> AppResult<Option<PathBuf>> {
    if prepared_is_empty {
        return Ok(None);
    }

    Ok(Some(
        create_backup(BackupRequest {
            installation: request.installation.clone(),
            output_path: request.backup_output_path.clone(),
            groups: vec![BackupGroup::Addons],
            label: Some("addon-lock-apply".to_string()),
        })?
        .archive_path,
    ))
}
