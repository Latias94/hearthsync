use super::*;

pub fn install_addon_from_index(
    request: AddonIndexInstallRequest,
) -> AppResult<AddonIndexInstallResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    install_addon_from_index_task(request, &cancellation, &mut progress)
}

pub fn install_addon_from_index_task<TCancel, TProgress>(
    request: AddonIndexInstallRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexInstallResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    install_addon_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn install_addon_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexInstallRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexInstallResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexInstall,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index install from `{}` into `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexInstall,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_install_with_provider(provider, request, cancellation, progress)?;
    let mut remapped_progress = RemappedTaskProgressSink {
        inner: progress,
        task: TaskKind::AddonIndexInstall,
    };
    let install =
        execute_install_plan_task(plan.install_plan, cancellation, &mut remapped_progress)
            .map_err(|error| {
                remap_cancelled_task_kind(
                    error,
                    TaskKind::AddonInstall,
                    TaskKind::AddonIndexInstall,
                )
            })?;

    Ok(AddonIndexInstallResult {
        index_path: plan.index_path,
        package: plan.package,
        install,
    })
}

fn prepare_index_install_with_provider<P>(
    provider: &P,
    request: AddonIndexInstallRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexInstallPlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let package = find_index_package(&index, &request.name)?.clone();
    ensure_package_supports_flavor(&package, request.installation.flavor.as_str())?;
    let resolved_source = resolve_index_package_source(&request.index_path, &package.source)?;
    let prepared = prepare_package_from_source_ref_task_with_provider(
        provider,
        PreparePackageFromSourceRefTaskRequest::new(
            &resolved_source,
            PreparePackageTaskContext::new(
                Some(request.installation.flavor),
                request.installation.platform,
                cancellation,
                TaskKind::AddonIndexInstall,
                TaskPhase::Preparing,
            ),
        ),
        progress,
    )?;
    let install_plan = prepare_install_prepared_addon(InstallPreparedAddonRequest {
        installation: request.installation,
        state_paths: request.state_paths,
        prepared,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        metadata: Some(metadata_from_index_package(&index, &package)),
    })?;

    Ok(IndexInstallPlan {
        index_path: request.index_path,
        package: package.clone(),
        install_plan,
    })
}
