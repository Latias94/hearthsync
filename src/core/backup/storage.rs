use std::fs;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use super::archive::{read_backup_metadata_from_path, restore_backup};
use super::model::{BackupCatalog, BackupCatalogEntry, RestoreBackupRequest, RestoredBackup};
use crate::core::error::{AppError, AppResult};
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressSink,
    emit_task_progress, ensure_task_not_cancelled,
};

pub fn list_backups(backup_dir: Option<&Path>) -> AppResult<BackupCatalog> {
    let backup_dir = resolve_backup_dir(backup_dir)?;
    if !backup_dir.exists() {
        return Ok(BackupCatalog {
            backup_dir,
            entries: Vec::new(),
        });
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&backup_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("zip"))
        {
            continue;
        }

        let metadata = read_backup_metadata_from_path(&path)?;
        let backup_id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                AppError::Validation(format!("invalid backup file name: {}", path.display()))
            })?
            .to_string();
        let archive_size_bytes = fs::metadata(&path)?.len();
        entries.push(BackupCatalogEntry {
            backup_id,
            archive_path: path,
            archive_size_bytes,
            metadata,
        });
    }

    entries.sort_by(|left, right| {
        right
            .metadata
            .created_at
            .cmp(&left.metadata.created_at)
            .then_with(|| right.archive_path.cmp(&left.archive_path))
    });

    Ok(BackupCatalog {
        backup_dir,
        entries,
    })
}

pub fn restore_backup_selection(request: RestoreBackupRequest) -> AppResult<RestoredBackup> {
    let cancellation = NeverCancel;
    let mut progress = NoopProgressSink;
    restore_backup_selection_task(request, &cancellation, &mut progress)
}

pub fn restore_backup_selection_task<TCancel, TProgress>(
    request: RestoreBackupRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<RestoredBackup>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    emit_task_progress(
        progress,
        TaskKind::BackupRestore,
        TaskPhase::Preparing,
        format!(
            "Resolving backup restore input {} for `{}`",
            describe_restore_input(&request),
            request.installation.flavor_root.display()
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::BackupRestore, TaskPhase::Preparing)?;

    let archive_path = resolve_backup_archive(
        request.archive_path.as_deref(),
        request.backup_id.as_deref(),
        request.backup_dir.as_deref(),
    )?;
    let metadata = read_backup_metadata_from_path(&archive_path)?;
    emit_task_progress(
        progress,
        TaskKind::BackupRestore,
        TaskPhase::BackingUp,
        format!(
            "Creating transactional restore checkpoint for group(s): {}",
            describe_groups(&metadata.groups)
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::BackupRestore, TaskPhase::BackingUp)?;
    emit_task_progress(
        progress,
        TaskKind::BackupRestore,
        TaskPhase::Executing,
        format!(
            "Restoring backup `{}` with group(s): {}",
            archive_path.display(),
            describe_groups(&metadata.groups)
        ),
    );
    ensure_task_not_cancelled(cancellation, TaskKind::BackupRestore, TaskPhase::Executing)?;

    let restored = restore_backup(&archive_path, &request.installation)?;
    emit_task_progress(
        progress,
        TaskKind::BackupRestore,
        TaskPhase::Completed,
        format!(
            "Backup restore completed with {} restored file(s)",
            restored.restored_files
        ),
    );
    Ok(restored)
}

pub(super) fn resolve_backup_dir(backup_dir: Option<&Path>) -> AppResult<PathBuf> {
    match backup_dir {
        Some(path) => Ok(path.to_path_buf()),
        None => default_backup_dir(),
    }
}

fn default_backup_dir() -> AppResult<PathBuf> {
    let project_dirs = ProjectDirs::from("dev", "hearthsync", "hearthsync").ok_or_else(|| {
        AppError::Validation("failed to determine platform-specific backup directory".to_string())
    })?;

    Ok(project_dirs.data_local_dir().join("backups"))
}

fn resolve_backup_archive(
    archive_path: Option<&Path>,
    backup_id: Option<&str>,
    backup_dir: Option<&Path>,
) -> AppResult<PathBuf> {
    match (archive_path, backup_id) {
        (Some(path), None) => Ok(path.to_path_buf()),
        (None, Some(backup_id)) => {
            let catalog = list_backups(backup_dir)?;
            let matched = catalog
                .entries
                .into_iter()
                .find(|entry| {
                    entry.backup_id == backup_id
                        || entry
                            .archive_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name == backup_id)
                })
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "backup `{backup_id}` not found in {}",
                        catalog.backup_dir.display()
                    ))
                })?;
            Ok(matched.archive_path)
        }
        (Some(_), Some(_)) => Err(AppError::Validation(
            "pass either `archive_path` or `backup_id`, not both".to_string(),
        )),
        (None, None) => Err(AppError::Validation(
            "either `archive_path` or `backup_id` is required".to_string(),
        )),
    }
}

fn describe_restore_input(request: &RestoreBackupRequest) -> String {
    match (&request.archive_path, &request.backup_id) {
        (Some(path), None) => format!("from archive `{}`", path.display()),
        (None, Some(backup_id)) => format!("from backup id `{backup_id}`"),
        (Some(_), Some(_)) => "with conflicting archive inputs".to_string(),
        (None, None) => "without a backup selector".to_string(),
    }
}

fn describe_groups(groups: &[super::model::BackupGroup]) -> String {
    if groups.is_empty() {
        "none".to_string()
    } else {
        groups
            .iter()
            .map(super::model::BackupGroup::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}
