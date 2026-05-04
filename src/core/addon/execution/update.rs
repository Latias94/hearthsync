use std::collections::BTreeSet;
use std::path::PathBuf;

use super::super::registry::registry_path;
use super::super::{
    AddonProvider, AddonRegistry, AddonStatePaths, DefaultAddonProvider,
    MissingDependencyCollectionRequest, MissingDependencyCollectionState,
    PreparePackageFromSourceRefTaskRequest, PreparePackageTaskContext, PreparedAddonPackage,
    TrackedAddonPackage, UpdateAddonRequest, UpdatePreparedPackagesWithDependenciesRequest,
    UpdatedAddonPackageResult, collect_missing_dependency_prepared_packages, load_registry,
    no_tracked_packages_error, policy::AddonUpdatePolicySnapshot,
    prepare_package_from_source_ref_task_with_provider, preview_installed_dependency_packages,
    rollback_or_report_addon_error, select_tracked_packages,
    update_prepared_packages_with_dependencies_task,
};
use super::backup::create_addon_backup;
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};
struct UpdateAddonsExecutionPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    registry: AddonRegistry,
    selected_packages: Vec<TrackedAddonPackage>,
    prepared_packages: Vec<PreparedAddonPackage>,
    dependency_prepared_packages: Vec<PreparedAddonPackage>,
    ignored_packages: Vec<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
    files_to_write: usize,
}

pub fn update_addons(request: UpdateAddonRequest) -> AppResult<UpdatedAddonPackageResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    update_addons_task(request, &cancellation, &mut progress)
}

pub fn update_addons_task<TCancel, TProgress>(
    request: UpdateAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UpdatedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    update_addons_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn update_addons_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: UpdateAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UpdatedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::Preparing,
        format!(
            "Preparing addon update for `{}`",
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonUpdate, TaskPhase::Preparing)?;

    let plan = prepare_update_addons_with_provider(provider, request, cancellation, progress)?;
    if plan.selected_packages.is_empty() {
        let result = no_op_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonUpdate,
            TaskPhase::Completed,
            format!(
                "Addon update completed without selected packages (ignored {} package(s))",
                result.ignored_packages.len()
            ),
        );
        return Ok(result);
    }
    if plan.dry_run {
        let result = dry_run_update_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonUpdate,
            TaskPhase::Completed,
            format!(
                "Addon update dry run completed with {} package(s) and {} pending file(s)",
                result.updated_packages.len(),
                result.files_to_write
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::BackingUp,
        "Creating AddOns backup before addon update",
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonUpdate, TaskPhase::BackingUp)?;
    let backup_path = create_addon_backup(
        &plan.installation,
        plan.backup_output_path.clone(),
        "addon-update",
    )?;

    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::Executing,
        update_execution_message(
            plan.selected_packages.len(),
            plan.dependency_prepared_packages.len(),
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonUpdate, TaskPhase::Executing)?;

    let result = execute_update_plan(plan, backup_path, cancellation, progress)?;
    emit_task_progress(
        progress,
        TaskKind::AddonUpdate,
        TaskPhase::Completed,
        format!(
            "Addon update completed with {} written file(s)",
            result.written_files
        ),
    );
    Ok(result)
}

fn prepare_update_addons_with_provider<P>(
    provider: &P,
    request: UpdateAddonRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<UpdateAddonsExecutionPlan>
where
    P: AddonProvider + ?Sized,
{
    let registry_path = registry_path(&request.state_paths);
    let registry = load_registry(&request.installation, &request.state_paths)?;
    if registry.packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let policies = AddonUpdatePolicySnapshot::load(&request.installation, &request.state_paths)?;
    let mut selected_packages = select_tracked_packages(&registry, request.name.as_deref())?;
    let ignored_packages = if request.name.is_some() {
        Vec::new()
    } else {
        let mut ignored = Vec::new();
        selected_packages.retain(|package| {
            if policies.is_ignored(package) {
                ignored.push(package.package_id.clone());
                false
            } else {
                true
            }
        });
        ignored.sort();
        ignored
    };
    let mut prepared_packages = Vec::new();
    let mut dependency_prepared_packages = Vec::new();
    let mut planned_dependency_keys = BTreeSet::new();
    for package in &selected_packages {
        let package_policy = policies.provider_update_policy(provider, package)?;
        let mut prepared = prepare_package_from_source_ref_task_with_provider(
            provider,
            PreparePackageFromSourceRefTaskRequest::new(
                &package_policy.effective_source,
                PreparePackageTaskContext::new(
                    Some(request.installation.flavor),
                    request.installation.platform,
                    cancellation,
                    TaskKind::AddonUpdate,
                    TaskPhase::Preparing,
                ),
            )
            .with_resolution_policy(package_policy.resolution_policy),
            progress,
        )?;
        prepared.package_id = package.package_id.clone();
        prepared_packages.push(prepared);

        if package_policy.install_dependencies {
            collect_missing_dependency_prepared_packages(
                provider,
                MissingDependencyCollectionRequest {
                    source: &package_policy.effective_source,
                    resolution_policy: package_policy.resolution_policy,
                    installation: &request.installation,
                    registry: &registry,
                    selected_packages: &selected_packages,
                    task_kind: TaskKind::AddonUpdate,
                },
                &mut MissingDependencyCollectionState {
                    prepared_packages: &mut dependency_prepared_packages,
                    planned_keys: &mut planned_dependency_keys,
                },
                cancellation,
                progress,
            )?;
        }
    }

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

    Ok(UpdateAddonsExecutionPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        registry,
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
        ignored_packages,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        registry_path,
        files_to_write,
    })
}

fn no_op_update_result(plan: UpdateAddonsExecutionPlan) -> UpdatedAddonPackageResult {
    UpdatedAddonPackageResult {
        dry_run: plan.dry_run,
        registry_path: plan.registry_path,
        files_to_write: 0,
        written_files: 0,
        updated_packages: Vec::new(),
        installed_dependency_packages: Vec::new(),
        ignored_packages: plan.ignored_packages,
        backup_path: None,
    }
}

fn dry_run_update_result(plan: UpdateAddonsExecutionPlan) -> UpdatedAddonPackageResult {
    let UpdateAddonsExecutionPlan {
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
        ignored_packages,
        registry_path,
        files_to_write,
        ..
    } = plan;

    UpdatedAddonPackageResult {
        dry_run: true,
        registry_path,
        files_to_write,
        written_files: 0,
        updated_packages: prepared_packages
            .into_iter()
            .zip(selected_packages.iter())
            .map(|(package, selected)| {
                let metadata = package
                    .metadata
                    .clone()
                    .or_else(|| selected.metadata.clone());
                TrackedAddonPackage {
                    package_id: package.package_id,
                    source: package.source,
                    installed_at: selected.installed_at.clone(),
                    updated_at: String::new(),
                    addons: package
                        .addons
                        .into_iter()
                        .map(|addon| addon.addon)
                        .collect(),
                    metadata,
                }
            })
            .collect(),
        installed_dependency_packages: preview_installed_dependency_packages(
            &dependency_prepared_packages,
        ),
        ignored_packages,
        backup_path: None,
    }
}

fn execute_update_plan<TCancel, TProgress>(
    plan: UpdateAddonsExecutionPlan,
    backup_path: PathBuf,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<UpdatedAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let UpdateAddonsExecutionPlan {
        installation,
        state_paths,
        registry,
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
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
            selected_packages,
            prepared_packages,
            dependency_prepared_packages,
            task: TaskKind::AddonUpdate,
        },
        cancellation,
        progress,
    ) {
        Ok((updated_packages, installed_dependency_packages, written_files)) => {
            Ok(UpdatedAddonPackageResult {
                dry_run: false,
                registry_path,
                files_to_write,
                written_files,
                updated_packages,
                installed_dependency_packages,
                ignored_packages,
                backup_path: Some(backup_path),
            })
        }
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}

fn update_execution_message(updated_count: usize, dependency_count: usize) -> String {
    match dependency_count {
        0 => format!("Updating {updated_count} tracked addon package(s)"),
        _ => format!(
            "Updating {updated_count} tracked addon package(s) and installing {dependency_count} dependency package(s)"
        ),
    }
}
