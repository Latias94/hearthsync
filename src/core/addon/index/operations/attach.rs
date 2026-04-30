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

fn prepare_index_attach_with_provider<P>(
    provider: &P,
    request: AddonIndexAttachRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<IndexAttachPlan>
where
    P: AddonProvider + ?Sized,
{
    let index = load_addon_index(&request.index_path)?;
    let inventory = list_addons(&request.installation, &request.state_paths)?;
    if inventory.tracked_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let registry = load_registry(&request.installation, &request.state_paths)?;
    let flavor = request.installation.flavor.as_str();
    let selected_packages = match request.name.as_deref() {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    let mut packages = Vec::new();
    let mut changes = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    let mut considered_package_count = 0usize;
    let mut skipped_unsupported_flavor_package_count = 0usize;

    for package in selected_packages {
        if let Err(error) = ensure_package_supports_flavor(&package, flavor) {
            if request.name.is_some() {
                return Err(error);
            }
            skipped_unsupported_flavor_package_count += 1;
            packages.push(skipped_unsupported_flavor_attach_result(
                package,
                error.to_string(),
            ));
            continue;
        }

        considered_package_count += 1;
        let package_for_matching =
            resolved_index_package_for_matching(&request.index_path, &package);
        let explained = match explain_preflight_match_index_package_to_tracked_package(
            &package_for_matching,
            &inventory.tracked_packages,
            &used_package_ids,
        ) {
            Ok(Some(matched)) => matched,
            Ok(None) => {
                packages.push(no_local_match_attach_result(package));
                continue;
            }
            Err(error) => {
                packages.push(ambiguous_local_match_attach_result(
                    package,
                    error.to_string(),
                ));
                continue;
            }
        };
        used_package_ids.insert(package_id_usage_key(&explained.package.package_id));
        let tracked_package = explained.package;
        let match_strategy = explained.strategy;

        let tracked_package_index = registry
            .packages
            .iter()
            .position(|candidate| *candidate == tracked_package)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could plan",
                    tracked_package.package_id
                ))
            })?;
        let resolved_source =
            match resolve_index_package_source(&request.index_path, &package.source) {
                Ok(source) => source,
                Err(error) => {
                    packages.push(prepare_failed_attach_result(
                        package,
                        Some(tracked_package.package_id.as_str()),
                        Some(match_strategy.clone()),
                        Some(tracked_package.source.clone()),
                        None,
                        error.to_string(),
                    ));
                    continue;
                }
            };
        let prepared = match prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &resolved_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    TaskKind::AddonIndexAttach,
                    TaskPhase::Preparing,
                ),
            ),
            progress,
        ) {
            Ok(prepared) => prepared,
            Err(error) => {
                packages.push(prepare_failed_attach_result(
                    package,
                    Some(tracked_package.package_id.as_str()),
                    Some(match_strategy.clone()),
                    Some(tracked_package.source.clone()),
                    Some(resolved_source),
                    error.to_string(),
                ));
                continue;
            }
        };
        if let Err(error) = ensure_relink_addon_directories_match(&tracked_package, &prepared) {
            packages.push(addon_directory_mismatch_attach_result(
                package,
                &tracked_package.package_id,
                match_strategy.clone(),
                tracked_package.source.clone(),
                prepared.source.clone(),
                error.to_string(),
            ));
            continue;
        }

        let metadata = metadata_from_index_package(&index, &package);
        let source_changed = relink_source_changed(&tracked_package, &prepared.source);
        let metadata_changed = tracked_package.metadata.as_ref() != Some(&metadata);
        if !source_changed && !metadata_changed {
            packages.push(already_attached_attach_result(
                package,
                &tracked_package.package_id,
                match_strategy,
                tracked_package.source.clone(),
            ));
            continue;
        }

        let package_result_index = packages.len();
        packages.push(ready_attach_result(ReadyAttachResultRequest {
            package: package.clone(),
            tracked_package_id: &tracked_package.package_id,
            match_strategy: match_strategy.clone(),
            previous_source: tracked_package.source.clone(),
            source: prepared.source.clone(),
            source_changed,
            metadata_changed,
            applied: false,
        }));
        changes.push(IndexAttachChange {
            package_result_index,
            package,
            tracked_package_index,
            tracked_package,
            next_source: prepared.source,
            metadata,
            match_strategy,
            source_changed,
            metadata_changed,
        });
    }

    Ok(IndexAttachPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        index_path: request.index_path,
        index_name: index.name,
        dry_run: request.dry_run,
        apply_ready_only: request.apply_ready_only,
        registry_path: inventory.registry_path,
        index_package_count: index.packages.len(),
        considered_package_count,
        skipped_unsupported_flavor_package_count,
        packages,
        changes,
    })
}

fn execute_index_attach_plan(plan: IndexAttachPlan) -> AppResult<AddonIndexAttachResult> {
    let mut registry = load_registry(&plan.installation, &plan.state_paths)?;
    let timestamp = relink_timestamp()?;

    for change in &plan.changes {
        let target = registry
            .packages
            .get(change.tracked_package_index)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could be applied",
                    change.tracked_package.package_id
                ))
            })?;
        if *target != change.tracked_package {
            return Err(AppError::Validation(format!(
                "tracked addon package `{}` changed before addon index attach could be applied",
                change.tracked_package.package_id
            )));
        }
    }

    for change in &plan.changes {
        let target = registry
            .packages
            .get_mut(change.tracked_package_index)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could be applied",
                    change.tracked_package.package_id
                ))
            })?;
        target.source = change.next_source.clone();
        target.updated_at = timestamp.clone();
        target.metadata = Some(change.metadata.clone());
    }
    save_registry(&plan.installation, &plan.state_paths, &registry)?;

    Ok(index_attach_result(plan, true))
}

fn index_attach_execute_message(plan: &IndexAttachPlan) -> String {
    let blocked_count = attach_blocked_package_count(&plan.packages);
    if blocked_count > 0 {
        format!(
            "Partially attaching {} ready curated addon index package(s) without reinstalling live AddOns; {} package(s) remain blocked",
            plan.changes.len(),
            blocked_count
        )
    } else {
        format!(
            "Attaching {} curated addon index package(s) without reinstalling live AddOns",
            plan.changes.len()
        )
    }
}

fn index_attach_result(mut plan: IndexAttachPlan, applied: bool) -> AddonIndexAttachResult {
    if applied {
        for change in &plan.changes {
            plan.packages[change.package_result_index] =
                ready_attach_result(ReadyAttachResultRequest {
                    package: change.package.clone(),
                    tracked_package_id: &change.tracked_package.package_id,
                    match_strategy: change.match_strategy.clone(),
                    previous_source: change.tracked_package.source.clone(),
                    source: change.next_source.clone(),
                    source_changed: change.source_changed,
                    metadata_changed: change.metadata_changed,
                    applied: true,
                });
        }
    }

    let change_package_count = plan.changes.len();
    let attached_package_count = if applied { change_package_count } else { 0 };
    let already_attached_package_count = plan
        .packages
        .iter()
        .filter(|package| {
            matches!(
                package.status,
                AddonIndexAttachPackageStatus::AlreadyAttached
            )
        })
        .count();
    let blocked_package_count = attach_blocked_package_count(&plan.packages);
    let partial_apply = applied && blocked_package_count > 0;

    AddonIndexAttachResult {
        index_path: plan.index_path,
        index_name: plan.index_name,
        dry_run: plan.dry_run,
        ready: blocked_package_count == 0,
        applied,
        partial_apply,
        registry_path: plan.registry_path,
        index_package_count: plan.index_package_count,
        considered_package_count: plan.considered_package_count,
        change_package_count,
        attached_package_count,
        already_attached_package_count,
        blocked_package_count,
        skipped_unsupported_flavor_package_count: plan.skipped_unsupported_flavor_package_count,
        packages: plan.packages,
    }
}

fn result_ready_for_attach(plan: &IndexAttachPlan) -> bool {
    attach_blocked_package_count(&plan.packages) == 0
}

fn attach_blocked_package_count(packages: &[AddonIndexAttachPackageResult]) -> usize {
    packages
        .iter()
        .filter(|package| attach_status_is_blocking(&package.status))
        .count()
}

fn attach_status_is_blocking(status: &AddonIndexAttachPackageStatus) -> bool {
    matches!(
        status,
        AddonIndexAttachPackageStatus::NoLocalMatch
            | AddonIndexAttachPackageStatus::AmbiguousLocalMatch
            | AddonIndexAttachPackageStatus::AddonDirectoryMismatch
            | AddonIndexAttachPackageStatus::PrepareFailed
    )
}

struct ReadyAttachResultRequest<'a> {
    package: AddonIndexPackage,
    tracked_package_id: &'a str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    previous_source: AddonSourceRef,
    source: AddonSourceRef,
    source_changed: bool,
    metadata_changed: bool,
    applied: bool,
}

fn ready_attach_result(request: ReadyAttachResultRequest<'_>) -> AddonIndexAttachPackageResult {
    let ReadyAttachResultRequest {
        package,
        tracked_package_id,
        match_strategy,
        previous_source,
        source,
        source_changed,
        metadata_changed,
        applied,
    } = request;
    let status = if applied {
        AddonIndexAttachPackageStatus::Attached
    } else {
        AddonIndexAttachPackageStatus::WouldAttach
    };

    AddonIndexAttachPackageResult {
        package,
        status,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy.clone()),
        previous_source: Some(previous_source),
        source: Some(source),
        source_changed,
        metadata_changed,
        message: attach_change_message(
            tracked_package_id,
            &match_strategy,
            source_changed,
            metadata_changed,
            applied,
        ),
    }
}

fn already_attached_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: &str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    source: AddonSourceRef,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AlreadyAttached,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy.clone()),
        previous_source: Some(source.clone()),
        source: Some(source),
        source_changed: false,
        metadata_changed: false,
        message: format!(
            "matched tracked package `{}` by {}; source and curated metadata already match",
            tracked_package_id,
            match_strategy_label(&match_strategy)
        ),
    }
}

fn no_local_match_attach_result(package: AddonIndexPackage) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::NoLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message: "no tracked addon package from the current registry matched this index package"
            .to_string(),
    }
}

fn ambiguous_local_match_attach_result(
    package: AddonIndexPackage,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AmbiguousLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn addon_directory_mismatch_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: &str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    previous_source: AddonSourceRef,
    source: AddonSourceRef,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AddonDirectoryMismatch,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy),
        previous_source: Some(previous_source),
        source: Some(source),
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn prepare_failed_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: Option<&str>,
    match_strategy: Option<AddonIndexTrackedMatchStrategy>,
    previous_source: Option<AddonSourceRef>,
    source: Option<AddonSourceRef>,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::PrepareFailed,
        matched_tracked_package_id: tracked_package_id.map(|value| value.to_string()),
        match_strategy,
        previous_source,
        source,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn skipped_unsupported_flavor_attach_result(
    package: AddonIndexPackage,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::SkippedUnsupportedFlavor,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn attach_change_message(
    tracked_package_id: &str,
    strategy: &AddonIndexTrackedMatchStrategy,
    source_changed: bool,
    metadata_changed: bool,
    applied: bool,
) -> String {
    let action = match (source_changed, metadata_changed, applied) {
        (true, true, true) => "attached curated source and metadata",
        (true, true, false) => "would attach curated source and metadata",
        (true, false, true) => "relinked the tracked source to the curated source",
        (true, false, false) => "would relink the tracked source to the curated source",
        (false, true, true) => "attached curated metadata",
        (false, true, false) => "would attach curated metadata",
        (false, false, true) => "left the tracked package unchanged",
        (false, false, false) => "would leave the tracked package unchanged",
    };

    format!(
        "matched tracked package `{}` by {}; {} without reinstalling live AddOns",
        tracked_package_id,
        match_strategy_label(strategy),
        action
    )
}

fn match_strategy_label(strategy: &AddonIndexTrackedMatchStrategy) -> &'static str {
    match strategy {
        AddonIndexTrackedMatchStrategy::StoredIndexPackageId => "stored index package id",
        AddonIndexTrackedMatchStrategy::ExactPackageId => "exact package id",
        AddonIndexTrackedMatchStrategy::CuratedMatchPackageId => "curated match_package_ids hint",
        AddonIndexTrackedMatchStrategy::SourceIdentity => "source identity",
        AddonIndexTrackedMatchStrategy::SourceFamilyIdentity => "source family identity",
        AddonIndexTrackedMatchStrategy::DisplayName => "display name",
        AddonIndexTrackedMatchStrategy::AddonDirectories => "addon directories",
        AddonIndexTrackedMatchStrategy::AddonDirectoryOverlap => "addon directory overlap",
    }
}
