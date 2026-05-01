use std::fs::{self, File};
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::ZipArchive;

use super::super::model::{
    BackupGroup, BackupMetadata, BackupRequest, CreatedBackup, RestoredBackup,
};
use super::create::create_backup;
use super::metadata::{read_backup_metadata, validate_backup_metadata_for_installation};
use crate::core::archive_io::{copy_reader_to_path, validate_zip_archive_entry};
use crate::core::archive_path::{
    PlatformPathCollisionKind, PlatformPathPrefixConflictKind, find_platform_path_collision,
    find_platform_path_prefix_conflict, join_segments, safe_zip_segments,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, TaskKind, TaskPhase, TaskProgressCode, TaskProgressSink,
    emit_task_step_progress, ensure_task_not_cancelled,
};
#[derive(Debug, Clone)]
struct PreparedRestoreArchive {
    archive_path: PathBuf,
    metadata: BackupMetadata,
    entries: Vec<PreparedRestoreEntry>,
}

#[derive(Debug, Clone)]
struct PreparedRestoreEntry {
    archive_index: usize,
    entry_name: String,
    destination: PathBuf,
}

#[derive(Debug)]
pub(super) struct ParsedBackupEntryTarget {
    pub(super) group: BackupGroup,
    pub(super) destination: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RestoreExecutionMode {
    Primary,
    Rollback,
}

#[derive(Clone, Copy)]
enum RestoreExecutionStep<'a> {
    ClearGroup {
        group: BackupGroup,
        current: usize,
        total: usize,
    },
    RestoreEntry {
        entry_name: &'a str,
        current: usize,
        total: usize,
    },
}

trait RestoreExecutionObserver {
    fn before_step(&mut self, _step: RestoreExecutionStep<'_>) -> AppResult<()> {
        Ok(())
    }
}

struct NoopRestoreExecutionObserver;

impl RestoreExecutionObserver for NoopRestoreExecutionObserver {}

struct TaskRestoreExecutionObserver<'a, TCancel, TProgress> {
    cancellation: &'a TCancel,
    progress: &'a mut TProgress,
}

impl<'a, TCancel, TProgress> TaskRestoreExecutionObserver<'a, TCancel, TProgress> {
    fn new(cancellation: &'a TCancel, progress: &'a mut TProgress) -> Self {
        Self {
            cancellation,
            progress,
        }
    }
}

impl<TCancel, TProgress> RestoreExecutionObserver
    for TaskRestoreExecutionObserver<'_, TCancel, TProgress>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    fn before_step(&mut self, step: RestoreExecutionStep<'_>) -> AppResult<()> {
        ensure_task_not_cancelled(
            self.cancellation,
            TaskKind::BackupRestore,
            TaskPhase::Executing,
        )?;
        if let Some((code, current, total, message)) = restore_execution_progress(step) {
            emit_task_step_progress(
                self.progress,
                TaskKind::BackupRestore,
                TaskPhase::Executing,
                code,
                current,
                total,
                message,
            );
        }
        Ok(())
    }
}

pub fn restore_backup(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<RestoredBackup> {
    let mut observer = NoopRestoreExecutionObserver;
    restore_backup_with_observer(archive_path, installation, &mut observer)
}

pub(in crate::core::backup) fn restore_backup_task<TCancel, TProgress>(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<RestoredBackup>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let mut observer = TaskRestoreExecutionObserver::new(cancellation, progress);
    restore_backup_with_observer(archive_path, installation, &mut observer)
}

fn restore_backup_with_observer(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
    observer: &mut impl RestoreExecutionObserver,
) -> AppResult<RestoredBackup> {
    let prepared = prepare_restore_archive(archive_path, installation)?;
    let rollback_stage = tempdir()?;
    let rollback_checkpoint =
        create_restore_checkpoint(&prepared.metadata, installation, rollback_stage.path())?;

    match apply_prepared_restore(
        &prepared,
        installation,
        RestoreExecutionMode::Primary,
        observer,
    ) {
        Ok(restored_files) => Ok(RestoredBackup {
            archive_path: prepared.archive_path,
            restored_files,
            metadata: prepared.metadata,
        }),
        Err(error) => {
            match restore_from_checkpoint(&rollback_checkpoint.archive_path, installation) {
                Ok(restored) => Err(AppError::Validation(format!(
                    "backup restore failed and transactional rollback restored pre-restore state from `{}` ({} files): {error}",
                    restored.archive_path.display(),
                    restored.restored_files
                ))),
                Err(rollback_error) => Err(AppError::Validation(format!(
                    "backup restore failed: {error}; transactional rollback failed: {rollback_error}"
                ))),
            }
        }
    }
}

fn prepare_restore_archive(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<PreparedRestoreArchive> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let metadata = read_backup_metadata(&mut archive)?;
    validate_backup_metadata_for_installation(&metadata, installation)?;

    let mut entries = Vec::new();

    for archive_index in 0..archive.len() {
        let entry = archive.by_index(archive_index)?;
        let entry_name = entry.name().to_string();
        validate_zip_archive_entry(
            "backup archive entry",
            &entry_name,
            entry.is_symlink(),
            entry.is_dir(),
        )?;
        if entry.is_dir() {
            continue;
        }
        if entry_name == "backup.toml" {
            continue;
        }

        let target = parse_backup_entry_target(&entry_name, installation)?;
        if !metadata.groups.contains(&target.group) {
            return Err(AppError::Validation(format!(
                "backup archive entry `{entry_name}` is not declared in backup metadata groups"
            )));
        }
        entries.push(PreparedRestoreEntry {
            archive_index,
            entry_name,
            destination: target.destination,
        });
    }

    if let Some(collision) =
        find_platform_path_collision(entries.iter(), installation.platform, |entry| {
            entry.destination.as_path()
        })
    {
        return match collision.kind {
            PlatformPathCollisionKind::Exact => Err(AppError::Validation(format!(
                "backup archive restores multiple entries onto the same destination: {}",
                collision.current.destination.display()
            ))),
            PlatformPathCollisionKind::CaseInsensitive => Err(AppError::Validation(format!(
                "backup archive contains case-insensitive restore destination collisions: `{}` -> {} and `{}` -> {} would map to the same path on Windows/default macOS targets",
                collision.previous.entry_name,
                collision.previous.destination.display(),
                collision.current.entry_name,
                collision.current.destination.display()
            ))),
        };
    }

    if let Some(conflict) =
        find_platform_path_prefix_conflict(entries.iter(), installation.platform, |entry| {
            entry.destination.as_path()
        })
    {
        return match conflict.kind {
            PlatformPathPrefixConflictKind::Exact => Err(AppError::Validation(format!(
                "backup archive contains conflicting restore destinations: {} and {}",
                conflict.ancestor.destination.display(),
                conflict.descendant.destination.display()
            ))),
            PlatformPathPrefixConflictKind::CaseInsensitive => Err(AppError::Validation(format!(
                "backup archive contains case-insensitive conflicting restore destinations: `{}` -> {} and `{}` -> {} would create file/directory collisions on Windows/default macOS targets",
                conflict.ancestor.entry_name,
                conflict.ancestor.destination.display(),
                conflict.descendant.entry_name,
                conflict.descendant.destination.display()
            ))),
        };
    }

    Ok(PreparedRestoreArchive {
        archive_path: archive_path.to_path_buf(),
        metadata,
        entries,
    })
}

fn clear_group_destination(
    group: BackupGroup,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    match group {
        BackupGroup::Addons => clear_directory(&installation.addon_dir),
        BackupGroup::Wtf => clear_directory(&installation.wtf_dir),
        BackupGroup::Fonts => clear_directory(&installation.fonts_dir),
        BackupGroup::InterfaceAssets => clear_interface_assets(&installation.interface_dir),
    }
}

fn clear_directory(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)?;
    Ok(())
}

fn clear_interface_assets(interface_dir: &Path) -> AppResult<()> {
    if !interface_dir.exists() {
        fs::create_dir_all(interface_dir)?;
        return Ok(());
    }

    for entry in fs::read_dir(interface_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case("AddOns") {
            continue;
        }

        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

pub(super) fn parse_backup_entry_target(
    entry_name: &str,
    installation: &DetectedFlavorInstallation,
) -> AppResult<ParsedBackupEntryTarget> {
    let segments = safe_zip_segments(entry_name)?;

    let Some((root, rest)) = segments.split_first() else {
        return Err(AppError::Validation(format!(
            "backup archive contains unsupported entry path: `{entry_name}`"
        )));
    };

    let Some(group) = BackupGroup::from_archive_root_name(root) else {
        return Err(AppError::Validation(format!(
            "backup archive contains unsupported root entry: `{entry_name}`"
        )));
    };

    if rest.is_empty() {
        return Err(AppError::Validation(format!(
            "backup archive contains unsupported entry path: `{entry_name}`"
        )));
    }

    Ok(ParsedBackupEntryTarget {
        group,
        destination: join_segments(group.installation_root(installation), rest),
    })
}

fn create_restore_checkpoint(
    metadata: &BackupMetadata,
    installation: &DetectedFlavorInstallation,
    output_root: &Path,
) -> AppResult<CreatedBackup> {
    create_backup(BackupRequest {
        installation: installation.clone(),
        output_path: Some(output_root.to_path_buf()),
        groups: metadata.groups.clone(),
        label: Some("restore-transaction".to_string()),
    })
}

fn restore_from_checkpoint(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<RestoredBackup> {
    let prepared = prepare_restore_archive(archive_path, installation)?;
    let mut observer = NoopRestoreExecutionObserver;
    let restored_files = apply_prepared_restore(
        &prepared,
        installation,
        RestoreExecutionMode::Rollback,
        &mut observer,
    )?;
    Ok(RestoredBackup {
        archive_path: prepared.archive_path,
        restored_files,
        metadata: prepared.metadata,
    })
}

fn apply_prepared_restore(
    prepared: &PreparedRestoreArchive,
    installation: &DetectedFlavorInstallation,
    mode: RestoreExecutionMode,
    observer: &mut impl RestoreExecutionObserver,
) -> AppResult<usize> {
    let groups = deduped_groups_in_order(&prepared.metadata.groups);
    let total_groups = groups.len();
    for (index, group) in groups.into_iter().enumerate() {
        observer.before_step(RestoreExecutionStep::ClearGroup {
            group,
            current: index + 1,
            total: total_groups,
        })?;
        clear_group_destination(group, installation)?;
    }

    let file = File::open(&prepared.archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut restored_files = 0usize;
    let total_entries = prepared.entries.len();
    for entry in &prepared.entries {
        maybe_inject_restore_test_failure(mode, restored_files)?;
        observer.before_step(RestoreExecutionStep::RestoreEntry {
            entry_name: &entry.entry_name,
            current: restored_files + 1,
            total: total_entries,
        })?;

        let mut archive_entry = archive.by_index(entry.archive_index)?;
        copy_reader_to_path(&mut archive_entry, &entry.destination).map_err(|error| {
            AppError::Validation(format!(
                "failed to copy backup entry `{}` to `{}`: {error}",
                entry.entry_name,
                entry.destination.display()
            ))
        })?;
        restored_files += 1;
    }

    Ok(restored_files)
}

fn deduped_groups_in_order(groups: &[BackupGroup]) -> Vec<BackupGroup> {
    let mut unique = Vec::new();
    for group in groups {
        if !unique.contains(group) {
            unique.push(*group);
        }
    }
    unique
}

fn restore_execution_progress(
    step: RestoreExecutionStep<'_>,
) -> Option<(TaskProgressCode, usize, usize, String)> {
    match step {
        RestoreExecutionStep::ClearGroup {
            group,
            current,
            total,
        } => Some((
            TaskProgressCode::ClearRestoreGroup,
            current,
            total,
            format!(
                "Clearing restore target group {current}/{total} `{}`",
                group.as_str()
            ),
        )),
        RestoreExecutionStep::RestoreEntry {
            entry_name,
            current,
            total,
        } if should_emit_restore_entry_progress(current, total) => Some((
            TaskProgressCode::RestoreEntry,
            current,
            total,
            format!("Restoring backup entry {current}/{total} `{entry_name}`"),
        )),
        RestoreExecutionStep::RestoreEntry { .. } => None,
    }
}

fn should_emit_restore_entry_progress(current: usize, total: usize) -> bool {
    current <= 3 || current == total || total <= 25 || current.is_multiple_of(25)
}

#[cfg(test)]
thread_local! {
    static TEST_RESTORE_FAIL_AFTER: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
fn maybe_inject_restore_test_failure(
    mode: RestoreExecutionMode,
    restored_files: usize,
) -> AppResult<()> {
    if mode != RestoreExecutionMode::Primary {
        return Ok(());
    }

    TEST_RESTORE_FAIL_AFTER.with(|fail_after| match fail_after.get() {
        Some(limit) if restored_files >= limit => Err(AppError::Validation(
            "injected restore failure for transaction test".to_string(),
        )),
        _ => Ok(()),
    })
}

#[cfg(not(test))]
fn maybe_inject_restore_test_failure(
    _mode: RestoreExecutionMode,
    _restored_files: usize,
) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
pub(in crate::core::backup) fn set_restore_test_failure_after(limit: Option<usize>) {
    TEST_RESTORE_FAIL_AFTER.with(|fail_after| fail_after.set(limit));
}
