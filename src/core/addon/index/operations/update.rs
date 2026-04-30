use super::*;

pub fn update_addons_from_index(
    request: AddonIndexUpdateRequest,
) -> AppResult<AddonIndexUpdateResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    update_addons_from_index_task(request, &cancellation, &mut progress)
}

pub fn update_addons_from_index_task<TCancel, TProgress>(
    request: AddonIndexUpdateRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexUpdateResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    update_addons_from_index_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn update_addons_from_index_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: AddonIndexUpdateRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexUpdateResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Preparing,
        format!(
            "Preparing addon index update from `{}` for `{}`",
            request.index_path.display(),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Preparing,
    )?;

    let plan = prepare_index_update_with_provider(provider, request, cancellation, progress)?;
    if plan.selected_packages.is_empty() {
        let result = no_op_index_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexUpdate,
            TaskPhase::Completed,
            format!(
                "Addon index update completed without selected packages (ignored {} package(s))",
                result.update.ignored_packages.len()
            ),
        );
        return Ok(result);
    }
    if plan.dry_run {
        let result = dry_run_index_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonIndexUpdate,
            TaskPhase::Completed,
            format!(
                "Addon index update dry run completed for {} package(s) with {} pending file(s)",
                result.selected_packages.len(),
                result.update.files_to_write
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::BackingUp,
        "Creating AddOns backup before addon index update",
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexUpdate,
        TaskPhase::BackingUp,
    )?;
    let backup_path =
        create_index_update_backup(&plan.installation, plan.backup_output_path.clone())?;

    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Executing,
        index_update_execution_message(
            plan.selected_packages.len(),
            plan.dependency_prepared_packages.len(),
        ),
    );
    ensure_task_not_cancelled(
        cancellation,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Executing,
    )?;
    let result = execute_index_update_plan(plan, backup_path, cancellation, progress)?;

    emit_task_progress(
        progress,
        TaskKind::AddonIndexUpdate,
        TaskPhase::Completed,
        format!(
            "Addon index update completed with {} written file(s)",
            result.update.written_files
        ),
    );
    Ok(result)
}

pub(crate) fn validate_addon_index_update_dependency_policy_support<P>(
    provider: &P,
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    index_path: &std::path::Path,
    name: Option<&str>,
) -> AppResult<()>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(index_path)?;
    let selected_packages = match name {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    for package in &selected_packages {
        ensure_package_supports_flavor(package, installation.flavor.as_str())?;
    }

    let inventory = list_addons(installation, state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Ok(());
    }

    let policies = AddonUpdatePolicySnapshot::load(installation, state_paths)?;
    let mut used_package_ids = BTreeSet::new();
    for package in &selected_packages {
        let Some(matched) = preflight_match_index_package_to_tracked_package(
            package,
            &inventory.tracked_packages,
            &used_package_ids,
        )?
        else {
            continue;
        };
        used_package_ids.insert(package_id_usage_key(&matched.package_id));

        let package_policy = policies.index_update_policy(&matched);
        if name.is_none() && package_policy.ignored {
            continue;
        }
        if !package_policy.install_dependencies {
            continue;
        }

        let resolved_source = resolve_index_package_source(index_path, &package.source)?;
        let _ = validate_dependency_resolution_support(provider, &resolved_source)?;
    }

    Ok(())
}

fn preview_updated_packages(
    matched_packages: &[TrackedAddonPackage],
    prepared_packages: &[PreparedAddonPackage],
) -> Vec<TrackedAddonPackage> {
    matched_packages
        .iter()
        .zip(prepared_packages.iter())
        .map(|(matched, prepared)| TrackedAddonPackage {
            package_id: prepared.package_id.clone(),
            source: prepared.source.clone(),
            installed_at: matched.installed_at.clone(),
            updated_at: String::new(),
            addons: prepared
                .addons
                .iter()
                .map(|addon| addon.addon.clone())
                .collect(),
            metadata: prepared
                .metadata
                .clone()
                .or_else(|| matched.metadata.clone()),
        })
        .collect()
}

fn prepare_index_update_with_provider<P>(
    provider: &P,
    request: AddonIndexUpdateRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexUpdatePlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let selected_packages = match &request.name {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    for package in &selected_packages {
        ensure_package_supports_flavor(package, request.installation.flavor.as_str())?;
    }

    let inventory = list_addons(&request.installation, &request.state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let registry = load_registry(&request.installation, &request.state_paths)?;
    let policies = AddonUpdatePolicySnapshot::load(&request.installation, &request.state_paths)?;
    let mut prepared_packages = Vec::new();
    let mut dependency_prepared_packages = Vec::new();
    let mut matched_packages = Vec::new();
    let mut effective_selected_packages = Vec::new();
    let mut ignored_packages = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    let mut planned_dependency_keys = BTreeSet::new();
    for package in &selected_packages {
        let preflight_matched = preflight_match_index_package_to_tracked_package(
            package,
            &inventory.tracked_packages,
            &used_package_ids,
        )?;
        if let Some(matched) = preflight_matched.as_ref() {
            let package_policy = policies.index_update_policy(matched);
            if request.name.is_none() && package_policy.ignored {
                used_package_ids.insert(package_id_usage_key(&matched.package_id));
                ignored_packages.push(matched.package_id.clone());
                continue;
            }
        }

        let resolved_source = resolve_index_package_source(&request.index_path, &package.source)?;
        let mut prepared = prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &resolved_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    TaskKind::AddonIndexUpdate,
                    TaskPhase::Preparing,
                ),
            ),
            progress,
        )?;
        prepared.metadata = Some(metadata_from_index_package(&index, package));
        let matched = match_index_package_to_tracked_package(
            package,
            &prepared,
            &inventory.tracked_packages,
            &used_package_ids,
        )?;
        used_package_ids.insert(package_id_usage_key(&matched.package_id));
        let package_policy = policies.index_update_policy(&matched);
        if request.name.is_none() && package_policy.ignored {
            ignored_packages.push(matched.package_id.clone());
            continue;
        }
        prepared.package_id = matched.package_id.clone();
        if package_policy.install_dependencies {
            if preflight_matched.is_none() {
                let _ = validate_dependency_resolution_support(provider, &resolved_source)
                    .map_err(|error| {
                        annotate_deferred_dependency_support_error(
                            error,
                            package,
                            &matched,
                            &resolved_source,
                        )
                    })?;
            }
            collect_missing_dependency_prepared_packages(
                provider,
                MissingDependencyCollectionRequest {
                    source: &resolved_source,
                    resolution_policy: AddonSourceResolutionPolicy::default(),
                    installation: &request.installation,
                    registry: &registry,
                    selected_packages: &matched_packages,
                    task_kind: TaskKind::AddonIndexUpdate,
                },
                &mut MissingDependencyCollectionState {
                    prepared_packages: &mut dependency_prepared_packages,
                    planned_keys: &mut planned_dependency_keys,
                },
                cancellation,
                progress,
            )?;
        }
        effective_selected_packages.push(package.clone());
        prepared_packages.push(prepared);
        matched_packages.push(matched);
    }
    ignored_packages.sort();

    let files_to_write = prepared_packages
        .iter()
        .chain(dependency_prepared_packages.iter())
        .map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.file_count)
                .sum::<usize>()
        })
        .sum::<usize>();

    Ok(IndexUpdatePlan {
        installation: request.installation,
        state_paths: request.state_paths,
        index_path: request.index_path,
        selected_packages: effective_selected_packages,
        registry,
        prepared_packages,
        dependency_prepared_packages,
        matched_packages,
        ignored_packages,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        registry_path: inventory.registry_path,
        files_to_write,
    })
}

fn no_op_index_update_result(plan: IndexUpdatePlan) -> AddonIndexUpdateResult {
    let IndexUpdatePlan {
        index_path,
        selected_packages,
        registry_path,
        ignored_packages,
        dry_run,
        ..
    } = plan;

    AddonIndexUpdateResult {
        index_path,
        selected_packages,
        update: UpdatedAddonPackageResult {
            dry_run,
            registry_path,
            files_to_write: 0,
            written_files: 0,
            updated_packages: Vec::new(),
            installed_dependency_packages: Vec::new(),
            ignored_packages,
            backup_path: None,
        },
    }
}

fn dry_run_index_update_result(plan: IndexUpdatePlan) -> AddonIndexUpdateResult {
    let IndexUpdatePlan {
        index_path,
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
        matched_packages,
        ignored_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    AddonIndexUpdateResult {
        index_path,
        selected_packages,
        update: UpdatedAddonPackageResult {
            dry_run: true,
            registry_path,
            files_to_write,
            written_files: 0,
            updated_packages: preview_updated_packages(&matched_packages, &prepared_packages),
            installed_dependency_packages: preview_installed_dependency_packages(
                &dependency_prepared_packages,
            ),
            ignored_packages,
            backup_path: None,
        },
    }
}

fn execute_index_update_plan<TCancel, TProgress>(
    plan: IndexUpdatePlan,
    backup_path: PathBuf,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<AddonIndexUpdateResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let IndexUpdatePlan {
        installation,
        state_paths,
        index_path,
        selected_packages,
        registry,
        prepared_packages,
        dependency_prepared_packages,
        matched_packages,
        ignored_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    match update_prepared_packages_with_dependencies_task(
        &installation,
        &state_paths,
        UpdatePreparedPackagesWithDependenciesRequest {
            registry,
            selected_packages: matched_packages,
            prepared_packages,
            dependency_prepared_packages,
            task: TaskKind::AddonIndexUpdate,
        },
        cancellation,
        progress,
    ) {
        Ok((updated_packages, installed_dependency_packages, written_files)) => {
            Ok(AddonIndexUpdateResult {
                index_path,
                selected_packages,
                update: UpdatedAddonPackageResult {
                    dry_run: false,
                    registry_path,
                    files_to_write,
                    written_files,
                    updated_packages,
                    installed_dependency_packages,
                    ignored_packages,
                    backup_path: Some(backup_path),
                },
            })
        }
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}

fn create_index_update_backup(
    installation: &DetectedFlavorInstallation,
    output_path: Option<PathBuf>,
) -> AppResult<PathBuf> {
    Ok(create_backup(BackupRequest {
        installation: installation.clone(),
        output_path,
        groups: vec![BackupGroup::Addons],
        label: Some("addon-index-update".to_string()),
    })?
    .archive_path)
}

fn index_update_execution_message(updated_count: usize, dependency_count: usize) -> String {
    match dependency_count {
        0 => format!("Updating {updated_count} addon index package(s)"),
        _ => format!(
            "Updating {updated_count} addon index package(s) and installing {dependency_count} dependency package(s)"
        ),
    }
}

fn annotate_deferred_dependency_support_error(
    error: AppError,
    package: &AddonIndexPackage,
    matched: &TrackedAddonPackage,
    source: &AddonSourceRef,
) -> AppError {
    match error {
        AppError::Validation(message) => AppError::Validation(format!(
            "{message} while updating addon index package `{}` matched to tracked package `{}`. addon-index app preflight could not determine this mapping from stable index identity hints alone, so the unsupported dependency policy surfaced only during domain update preparation. Consider adding exact `match_package_ids`, stable `addon_directories`, or unique exact package-name continuity for source `{}`.",
            package.id,
            matched.package_id,
            source.display_name()
        )),
        other => other,
    }
}
