use std::path::PathBuf;

use super::super::find_existing_addon_path;
use super::super::registry::registry_path;
use super::super::{
    AddonPackageMetadata, AddonProvider, AddonStatePaths, DefaultAddonProvider,
    InstallAddonRequest, InstalledAddonPackageResult, PreparePackageFromSourceInputTaskRequest,
    PreparePackageTaskContext, PreparedAddonPackage, install_prepared_package_task,
    prepare_package_from_source_input_task_with_provider, rollback_or_report_addon_error,
};
use super::backup::create_addon_backup;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};
#[derive(Debug)]
pub(crate) struct InstallPreparedAddonRequest {
    pub(crate) installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub(crate) prepared: PreparedAddonPackage,
    pub(crate) dry_run: bool,
    pub(crate) backup_output_path: Option<PathBuf>,
    pub(crate) replace_existing: bool,
    pub(crate) metadata: Option<AddonPackageMetadata>,
}

pub(crate) struct InstallAddonExecutionPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    prepared: PreparedAddonPackage,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    replace_existing: bool,
    registry_path: PathBuf,
    files_to_write: usize,
    replaced_addons: Vec<String>,
}

pub fn install_addon(request: InstallAddonRequest) -> AppResult<InstalledAddonPackageResult> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    install_addon_task(request, &cancellation, &mut progress)
}

pub fn install_addon_task<TCancel, TProgress>(
    request: InstallAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let provider = DefaultAddonProvider::default();
    install_addon_task_with_provider(&provider, request, cancellation, progress)
}

pub(crate) fn install_addon_task_with_provider<TCancel, TProgress, P>(
    provider: &P,
    request: InstallAddonRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
    P: AddonProvider + ?Sized,
{
    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::Preparing,
        format!(
            "Preparing addon installation from `{}` into `{}`",
            request.source,
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonInstall, TaskPhase::Preparing)?;

    let plan = prepare_install_addon_with_provider(provider, request, cancellation, progress)?;
    execute_install_plan_task(plan, cancellation, progress)
}

pub(crate) fn execute_install_plan_task<TCancel, TProgress>(
    plan: InstallAddonExecutionPlan,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    if plan.dry_run {
        let result = dry_run_install_result(plan);
        emit_task_progress(
            progress,
            TaskKind::AddonInstall,
            TaskPhase::Completed,
            format!(
                "Addon install dry run completed with {} addon(s) and {} pending file(s)",
                result.addons.len(),
                result.files_to_write
            ),
        );
        return Ok(result);
    }

    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::BackingUp,
        "Creating AddOns backup before addon install",
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonInstall, TaskPhase::BackingUp)?;
    let backup_path = create_addon_backup(
        &plan.installation,
        plan.backup_output_path.clone(),
        "addon-install",
    )?;

    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::Executing,
        format!(
            "Installing {} addon directory(s)",
            plan.prepared.addons.len()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::AddonInstall, TaskPhase::Executing)?;

    let result = execute_install_plan(plan, backup_path, cancellation, progress)?;
    emit_task_progress(
        progress,
        TaskKind::AddonInstall,
        TaskPhase::Completed,
        format!(
            "Addon install completed with {} written file(s)",
            result.written_files
        ),
    );
    Ok(result)
}

fn prepare_install_addon_with_provider<P>(
    provider: &P,
    request: InstallAddonRequest,
    cancellation: &dyn CancellationToken,
    progress: &mut impl TaskProgressSink,
) -> AppResult<InstallAddonExecutionPlan>
where
    P: AddonProvider + ?Sized,
{
    let prepared = prepare_package_from_source_input_task_with_provider(
        provider,
        PreparePackageFromSourceInputTaskRequest {
            source: &request.source,
            context: PreparePackageTaskContext::new(
                Some(request.installation.flavor),
                request.installation.platform,
                cancellation,
                TaskKind::AddonInstall,
                TaskPhase::Preparing,
            ),
        },
        progress,
    )?;
    prepare_install_prepared_addon(InstallPreparedAddonRequest {
        installation: request.installation,
        state_paths: request.state_paths,
        prepared,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        metadata: request.metadata,
    })
}

pub(crate) fn prepare_install_prepared_addon(
    request: InstallPreparedAddonRequest,
) -> AppResult<InstallAddonExecutionPlan> {
    let registry_path = registry_path(&request.state_paths);
    let mut prepared = request.prepared;
    prepared.metadata = request.metadata;
    let files_to_write = prepared
        .addons
        .iter()
        .map(|addon| addon.file_count)
        .sum::<usize>();
    let replaced_addons = prepared
        .addons
        .iter()
        .filter_map(|addon| {
            find_existing_addon_path(
                &request.installation.addon_dir,
                &addon.addon.directory_name,
                request.installation.platform,
            )
            .transpose()
        })
        .map(|existing| existing.map(|existing| existing.name))
        .collect::<AppResult<Vec<_>>>()?;

    if !request.replace_existing && !replaced_addons.is_empty() {
        return Err(AppError::Validation(format!(
            "addon directories already exist: {}. Use `--replace-existing` or `addon update`.",
            replaced_addons.join(", ")
        )));
    }

    Ok(InstallAddonExecutionPlan {
        installation: request.installation,
        state_paths: request.state_paths,
        prepared,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        registry_path,
        files_to_write,
        replaced_addons,
    })
}

fn dry_run_install_result(plan: InstallAddonExecutionPlan) -> InstalledAddonPackageResult {
    let InstallAddonExecutionPlan {
        prepared,
        files_to_write,
        replaced_addons,
        registry_path,
        ..
    } = plan;
    let PreparedAddonPackage {
        source,
        package_id,
        addons,
        ..
    } = prepared;

    InstalledAddonPackageResult {
        dry_run: true,
        source,
        package_id,
        addons: addons.into_iter().map(|addon| addon.addon).collect(),
        files_to_write,
        written_files: 0,
        replaced_addons,
        registry_path,
        backup_path: None,
    }
}

fn execute_install_plan<TCancel, TProgress>(
    plan: InstallAddonExecutionPlan,
    backup_path: PathBuf,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<InstalledAddonPackageResult>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let InstallAddonExecutionPlan {
        installation,
        state_paths,
        prepared,
        replace_existing,
        registry_path,
        files_to_write,
        replaced_addons,
        ..
    } = plan;

    match install_prepared_package_task(
        &installation,
        &state_paths,
        prepared,
        replace_existing,
        TaskKind::AddonInstall,
        cancellation,
        progress,
    ) {
        Ok((package, written_files)) => Ok(InstalledAddonPackageResult {
            dry_run: false,
            source: package.source.clone(),
            package_id: package.package_id.clone(),
            addons: package.addons.clone(),
            files_to_write,
            written_files,
            replaced_addons,
            registry_path,
            backup_path: Some(backup_path),
        }),
        Err(error) => {
            rollback_or_report_addon_error(error, Some(backup_path.as_path()), &installation)
        }
    }
}
