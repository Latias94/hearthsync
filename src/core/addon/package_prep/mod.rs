mod archive;
mod inspect;
mod package_id;

use std::cell::RefCell;
use std::path::Path;

use tempfile::{TempDir, tempdir};

use super::provider::{
    AddonDownloadProgressObserver, AddonProvider, AddonProviderContext,
    AddonSourceResolutionPolicy, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
};
use super::{AddonSourceRef, PreparedAddonDirectory, PreparedAddonPackage, TrackedAddon};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{
    CancellationToken, TaskKind, TaskPhase, TaskProgressCode, TaskProgressSink,
    emit_task_byte_progress,
};

use self::archive::extract_archive_addons;
pub(crate) use self::inspect::{find_primary_toc, inspect_addon_directory};
use self::package_id::derive_package_id;
pub(crate) use self::package_id::slugify_package_id;

pub(crate) fn prepare_package_from_source_input_with_provider<P>(
    provider: &P,
    source: &str,
    target_flavor: Option<WowFlavor>,
    target_platform: HostPlatform,
    cancellation: &dyn CancellationToken,
) -> AppResult<PreparedAddonPackage>
where
    P: AddonProvider + ?Sized,
{
    let stage_dir = tempdir()?;
    let materialized = provider.materialize_source_input(MaterializeSourceInputRequest {
        source,
        stage_root: stage_dir.path(),
        context: AddonProviderContext::new(target_flavor, Some(cancellation)),
    })?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        target_platform,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_source_input_task_with_provider<P, TProgress>(
    provider: &P,
    source: &str,
    target_flavor: Option<WowFlavor>,
    target_platform: HostPlatform,
    cancellation: &dyn CancellationToken,
    task: TaskKind,
    phase: TaskPhase,
    progress: &mut TProgress,
) -> AppResult<PreparedAddonPackage>
where
    P: AddonProvider + ?Sized,
    TProgress: TaskProgressSink,
{
    let stage_dir = tempdir()?;
    let download_progress = TaskAddonDownloadProgressObserver::new(task, phase, progress);
    let materialized = provider.materialize_source_input(MaterializeSourceInputRequest {
        source,
        stage_root: stage_dir.path(),
        context: AddonProviderContext::new(target_flavor, Some(cancellation))
            .with_download_progress(Some(&download_progress)),
    })?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        target_platform,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_source_ref_task_with_provider<P, TProgress>(
    provider: &P,
    source: &AddonSourceRef,
    target_flavor: Option<WowFlavor>,
    target_platform: HostPlatform,
    cancellation: &dyn CancellationToken,
    task: TaskKind,
    phase: TaskPhase,
    progress: &mut TProgress,
) -> AppResult<PreparedAddonPackage>
where
    P: AddonProvider + ?Sized,
    TProgress: TaskProgressSink,
{
    prepare_package_from_source_ref_task_with_provider_and_policy(
        provider,
        source,
        AddonSourceResolutionPolicy::default(),
        target_flavor,
        target_platform,
        cancellation,
        task,
        phase,
        progress,
    )
}

pub(crate) fn prepare_package_from_source_ref_task_with_provider_and_policy<P, TProgress>(
    provider: &P,
    source: &AddonSourceRef,
    resolution_policy: AddonSourceResolutionPolicy,
    target_flavor: Option<WowFlavor>,
    target_platform: HostPlatform,
    cancellation: &dyn CancellationToken,
    task: TaskKind,
    phase: TaskPhase,
    progress: &mut TProgress,
) -> AppResult<PreparedAddonPackage>
where
    P: AddonProvider + ?Sized,
    TProgress: TaskProgressSink,
{
    let stage_dir = tempdir()?;
    let download_progress = TaskAddonDownloadProgressObserver::new(task, phase, progress);
    let materialized = provider.materialize_source_ref(MaterializeSourceRefRequest {
        source,
        stage_root: stage_dir.path(),
        context: AddonProviderContext::new(target_flavor, Some(cancellation))
            .with_resolution_policy(resolution_policy)
            .with_download_progress(Some(&download_progress)),
    })?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        target_platform,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_archive_with_source(
    source: AddonSourceRef,
    archive_path: &Path,
    target_platform: HostPlatform,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    prepare_package_from_archive(source, archive_path, target_platform, stage_dir)
}

fn prepare_package_from_archive(
    source: AddonSourceRef,
    archive_path: &Path,
    target_platform: HostPlatform,
    stage_dir: TempDir,
) -> AppResult<PreparedAddonPackage> {
    let addons = extract_archive_addons(archive_path, stage_dir.path(), target_platform)?;
    if addons.is_empty() {
        return Err(AppError::Validation(
            "archive does not contain any detectable addon directories".to_string(),
        ));
    }

    let addon_names = addons
        .iter()
        .map(|addon| addon.addon.directory_name.as_str())
        .collect::<Vec<_>>();
    let package_id = derive_package_id(&source, &addon_names);

    Ok(PreparedAddonPackage {
        source,
        package_id,
        addons,
        metadata: None,
        _stage_dir: stage_dir,
    })
}

struct TaskAddonDownloadProgressObserver<'a, TProgress> {
    task: TaskKind,
    phase: TaskPhase,
    progress: RefCell<&'a mut TProgress>,
}

impl<'a, TProgress> TaskAddonDownloadProgressObserver<'a, TProgress> {
    fn new(task: TaskKind, phase: TaskPhase, progress: &'a mut TProgress) -> Self {
        Self {
            task,
            phase,
            progress: RefCell::new(progress),
        }
    }
}

impl<TProgress> AddonDownloadProgressObserver for TaskAddonDownloadProgressObserver<'_, TProgress>
where
    TProgress: TaskProgressSink,
{
    fn on_download_progress(
        &self,
        _source: &AddonSourceRef,
        archive_name: &str,
        bytes_current: u64,
        bytes_total: Option<u64>,
        bytes_per_second: Option<u64>,
    ) {
        let mut progress = self.progress.borrow_mut();
        emit_task_byte_progress(
            &mut **progress,
            self.task,
            self.phase,
            TaskProgressCode::DownloadArchive,
            bytes_current,
            bytes_total,
            bytes_per_second,
            download_progress_message(archive_name, bytes_current, bytes_total, bytes_per_second),
        );
    }
}

fn download_progress_message(
    archive_name: &str,
    bytes_current: u64,
    bytes_total: Option<u64>,
    bytes_per_second: Option<u64>,
) -> String {
    let archive_name = if archive_name.trim().is_empty() {
        "downloaded-addon.zip"
    } else {
        archive_name
    };

    match (bytes_total, bytes_per_second) {
        (Some(bytes_total), Some(bytes_per_second)) => format!(
            "Downloading addon archive `{archive_name}` ({} / {}, {}/s)",
            format_byte_count(bytes_current),
            format_byte_count(bytes_total),
            format_byte_count(bytes_per_second)
        ),
        (Some(bytes_total), None) => format!(
            "Downloading addon archive `{archive_name}` ({} / {})",
            format_byte_count(bytes_current),
            format_byte_count(bytes_total)
        ),
        (None, Some(bytes_per_second)) => format!(
            "Downloading addon archive `{archive_name}` ({}, {}/s)",
            format_byte_count(bytes_current),
            format_byte_count(bytes_per_second)
        ),
        (None, None) => format!(
            "Downloading addon archive `{archive_name}` ({})",
            format_byte_count(bytes_current)
        ),
    }
}

fn format_byte_count(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit_index = 0usize;
    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    format!("{value:.1} {}", UNITS[unit_index])
}
