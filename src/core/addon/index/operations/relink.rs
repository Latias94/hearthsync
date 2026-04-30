use super::*;

pub fn relink_addon_from_index(
    request: AddonIndexRelinkRequest,
) -> AppResult<AddonIndexRelinkResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    relink_addon_from_index_task(request, &cancellation, &mut progress)
}

pub fn relink_addon_from_index_task<TCancel, TProgress>(
    request: AddonIndexRelinkRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexRelinkResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    relink_addon_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn relink_addon_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexRelinkRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexRelinkResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexRelink,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index relink from `{}` for `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexRelink,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_relink_with_provider(provider, request, cancellation, progress)?;
    if !plan.source_changed && !plan.metadata_changed {
        return Err(AppError::Validation(format!(
            "tracked addon package `{}` is already linked to index package `{}` from `{}`",
            plan.tracked_package.package_id, plan.package.id, plan.index_name
        )));
    }

    if plan.dry_run {
        let result = dry_run_index_relink_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexRelink,
            TaskPhase::Completed,
            format!(
                "Addon index relink dry run completed for `{}` (source changed: {}, metadata changed: {})",
                result.tracked_package_id, result.source_changed, result.metadata_changed
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonIndexRelink,
        TaskPhase::Executing,
        format!(
            "Relinking tracked package `{}` to index package `{}`",
            plan.tracked_package.package_id, plan.package.id
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexRelink,
        TaskPhase::Executing,
    )?;

    let result = execute_index_relink_plan(plan)?;
    emit_task_progress(
        progress,
        TaskKind::AddonIndexRelink,
        TaskPhase::Completed,
        format!(
            "Addon index relink completed for `{}` (source changed: {}, metadata changed: {})",
            result.tracked_package_id, result.source_changed, result.metadata_changed
        ),
    );
    Ok(result)
}

fn prepare_index_relink_with_provider<P>(
    provider: &P,
    request: AddonIndexRelinkRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexRelinkPlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let package = find_index_package(&index, &request.name)?.clone();
    ensure_package_supports_flavor(&package, request.installation.flavor.as_str())?;

    let inventory = list_addons(&request.installation, &request.state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let registry = load_registry(&request.installation, &request.state_paths)?;
    let resolved_source = resolve_index_package_source(&request.index_path, &package.source)?;
    let prepared = prepare_package_from_source_ref_task_with_provider(
        provider,
        PreparePackageFromSourceRefTaskRequest::new(
            &resolved_source,
            PreparePackageTaskContext::new(
                Some(request.installation.flavor),
                request.installation.platform,
                cancellation,
                TaskKind::AddonIndexRelink,
                TaskPhase::Preparing,
            ),
        ),
        progress,
    )?;

    let (tracked_package_index, tracked_package) = match request.target.as_deref() {
        Some(target) if target.trim().is_empty() => {
            return Err(AppError::Validation(
                "tracked addon selector for addon index relink must not be empty".to_string(),
            ));
        }
        Some(target) => select_single_tracked_package(&registry, target.trim())?,
        None => {
            let matched = match_index_package_to_tracked_package(
                &package,
                &prepared,
                &inventory.tracked_packages,
                &BTreeSet::new(),
            )?;
            let tracked_package_index = registry
                .packages
                .iter()
                .position(|candidate| *candidate == matched)
                .ok_or_else(|| {
                    AppError::Validation(format!(
                        "tracked addon package `{}` disappeared before addon index relink",
                        matched.package_id
                    ))
                })?;
            (tracked_package_index, matched)
        }
    };
    ensure_relink_addon_directories_match(&tracked_package, &prepared)?;

    let metadata = metadata_from_index_package(&index, &package);
    let source_changed = relink_source_changed(&tracked_package, &prepared.source);
    let metadata_changed = tracked_package.metadata.as_ref() != Some(&metadata);

    Ok(IndexRelinkPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        index_path: request.index_path,
        index_name: index.name,
        package,
        tracked_package_index,
        tracked_package,
        next_source: prepared.source,
        metadata,
        dry_run: request.dry_run,
        registry_path: inventory.registry_path,
        source_changed,
        metadata_changed,
    })
}

fn dry_run_index_relink_result(plan: IndexRelinkPlan) -> AddonIndexRelinkResult {
    AddonIndexRelinkResult {
        index_path: plan.index_path,
        package: plan.package,
        dry_run: true,
        tracked_package_id: plan.tracked_package.package_id,
        previous_source: plan.tracked_package.source,
        source: plan.next_source,
        addons: plan.tracked_package.addons,
        metadata: plan.metadata,
        registry_path: plan.registry_path,
        source_changed: plan.source_changed,
        metadata_changed: plan.metadata_changed,
    }
}

fn execute_index_relink_plan(plan: IndexRelinkPlan) -> AppResult<AddonIndexRelinkResult> {
    let IndexRelinkPlan {
        installation,
        state_paths,
        index_path,
        package,
        tracked_package_index,
        tracked_package,
        next_source,
        metadata,
        registry_path,
        source_changed,
        metadata_changed,
        ..
    } = plan;

    let mut registry = load_registry(&installation, &state_paths)?;
    let target = registry
        .packages
        .get_mut(tracked_package_index)
        .ok_or_else(|| {
            AppError::Validation(format!(
                "tracked addon package `{}` disappeared before addon index relink",
                tracked_package.package_id
            ))
        })?;

    if *target != tracked_package {
        return Err(AppError::Validation(format!(
            "tracked addon package `{}` changed before addon index relink could be applied",
            tracked_package.package_id
        )));
    }

    target.source = next_source.clone();
    target.updated_at = relink_timestamp()?;
    target.metadata = Some(metadata.clone());
    save_registry(&installation, &state_paths, &registry)?;

    Ok(AddonIndexRelinkResult {
        index_path,
        package,
        dry_run: false,
        tracked_package_id: tracked_package.package_id,
        previous_source: tracked_package.source,
        source: next_source,
        addons: tracked_package.addons,
        metadata,
        registry_path,
        source_changed,
        metadata_changed,
    })
}
