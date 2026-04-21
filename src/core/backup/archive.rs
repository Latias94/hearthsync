use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::model::{BackupGroup, BackupMetadata, BackupRequest, CreatedBackup, RestoredBackup};
use super::storage::resolve_backup_dir;
use crate::core::archive_io::{
    PortableArchivePathSet, add_directory_to_zip, copy_reader_to_path,
    portable_archive_path_issue_error, reject_unsupported_symlink_metadata, start_file_to_zip,
    stream_file_to_zip, validate_zip_archive_entry,
};
use crate::core::archive_path::{
    PlatformPathCollisionKind, PlatformPathPrefixConflictKind, find_platform_path_collision,
    find_platform_path_prefix_conflict, join_segments, safe_zip_segments, to_zip_path,
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
struct ParsedBackupEntryTarget {
    group: BackupGroup,
    destination: PathBuf,
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

pub fn create_backup(request: BackupRequest) -> AppResult<CreatedBackup> {
    if request.groups.is_empty() {
        return Err(AppError::Validation(
            "backup request must include at least one group".to_string(),
        ));
    }

    let timestamp = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let output_dir = resolve_backup_dir(request.output_path.as_deref())?;

    fs::create_dir_all(&output_dir)?;

    let file_name = build_backup_file_name(
        request.installation.flavor.as_str(),
        request.label.as_deref(),
        &timestamp,
    );
    let archive_path = output_dir.join(file_name);
    let file = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archive_outputs = PortableArchivePathSet::new();
    let mut archived_files = 0usize;

    for group in &request.groups {
        archived_files += add_backup_group_to_zip(
            &mut zip,
            &request.installation,
            *group,
            &mut archive_outputs,
        )?;
    }

    let metadata = BackupMetadata {
        schema_version: 1,
        created_at: timestamp,
        label: request
            .label
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        flavor: request.installation.flavor.as_str().to_string(),
        flavor_root: request.installation.flavor_root.clone(),
        groups: request.groups,
    };

    register_backup_archive_output(&mut archive_outputs, "backup.toml", false)?;
    start_file_to_zip(&mut zip, "backup.toml", zip_file_options())?;
    zip.write_all(toml::to_string_pretty(&metadata)?.as_bytes())?;
    zip.finish()?;

    Ok(CreatedBackup {
        archive_path,
        archived_files,
        metadata,
    })
}

pub fn restore_backup(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<RestoredBackup> {
    let mut observer = NoopRestoreExecutionObserver;
    restore_backup_with_observer(archive_path, installation, &mut observer)
}

pub(crate) fn restore_backup_task<TCancel, TProgress>(
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

pub(super) fn read_backup_metadata_from_path(path: &Path) -> AppResult<BackupMetadata> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    read_backup_metadata(&mut archive)
}

fn prepare_restore_archive(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
) -> AppResult<PreparedRestoreArchive> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let metadata = read_backup_metadata(&mut archive)?;
    validate_backup_metadata(&metadata, installation)?;

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

fn build_backup_file_name(flavor: &str, label: Option<&str>, timestamp: &str) -> String {
    let compact_timestamp = timestamp
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect::<String>();

    match label {
        Some(value) if !value.trim().is_empty() => {
            format!("backup-{flavor}-{value}-{compact_timestamp}.zip")
        }
        _ => format!("backup-{flavor}-{compact_timestamp}.zip"),
    }
}

fn add_backup_group_to_zip(
    zip: &mut ZipWriter<File>,
    installation: &DetectedFlavorInstallation,
    group: BackupGroup,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    match group {
        BackupGroup::InterfaceAssets => {
            add_interface_assets_group(zip, group.installation_root(installation), archive_outputs)
        }
        _ => add_directory_group(
            zip,
            group.installation_root(installation),
            Path::new(group.archive_root_name()),
            archive_outputs,
        ),
    }
}

fn add_directory_group(
    zip: &mut ZipWriter<File>,
    source_dir: &Path,
    archive_root: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    if !source_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in WalkDir::new(source_dir).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let file_type = entry.file_type();
        let path = entry.path();
        reject_unsupported_backup_source_symlink("directory", path, file_type.is_symlink())?;
        let relative = path
            .strip_prefix(source_dir)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        let archive_path = archive_root.join(relative);

        if file_type.is_dir() {
            let archive_name = to_zip_path(&archive_path);
            register_backup_archive_output(archive_outputs, &archive_name, true)?;
            add_directory_to_zip(zip, &archive_name, zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &archive_path, archive_outputs)?;
        archived_files += 1;
    }

    Ok(archived_files)
}

fn add_interface_assets_group(
    zip: &mut ZipWriter<File>,
    interface_dir: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    if !interface_dir.exists() {
        return Ok(0);
    }

    let mut archived_files = 0usize;

    for entry in fs::read_dir(interface_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let path = entry.path();
        reject_unsupported_backup_source_symlink("interface asset", &path, file_type.is_symlink())?;
        let name = entry.file_name();
        if name.to_string_lossy().eq_ignore_ascii_case("AddOns") {
            continue;
        }

        let archive_root = Path::new("interface").join(name);
        if file_type.is_dir() {
            archived_files += add_directory_group(zip, &path, &archive_root, archive_outputs)?;
        } else if file_type.is_file() {
            write_file_to_zip(zip, &path, &archive_root, archive_outputs)?;
            archived_files += 1;
        }
    }

    Ok(archived_files)
}

fn write_file_to_zip(
    zip: &mut ZipWriter<File>,
    source_path: &Path,
    archive_path: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<()> {
    let archive_name = to_zip_path(archive_path);
    register_backup_archive_output(archive_outputs, &archive_name, false)?;
    stream_file_to_zip(zip, source_path, &archive_name, zip_file_options())
}

fn register_backup_archive_output(
    archive_outputs: &mut PortableArchivePathSet,
    archive_path: &str,
    is_directory: bool,
) -> AppResult<()> {
    archive_outputs
        .register(archive_path, is_directory)
        .map_err(|issue| portable_archive_path_issue_error("backup creation", issue))
}

fn read_backup_metadata(archive: &mut ZipArchive<File>) -> AppResult<BackupMetadata> {
    let mut entry = archive.by_name("backup.toml")?;
    let mut content = String::new();
    entry.read_to_string(&mut content)?;
    Ok(toml::from_str(&content)?)
}

fn validate_backup_metadata(
    metadata: &BackupMetadata,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    if metadata.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported backup schema version: {}",
            metadata.schema_version
        )));
    }

    if metadata.groups.is_empty() {
        return Err(AppError::Validation(
            "backup metadata must include at least one group".to_string(),
        ));
    }

    let mut groups = BTreeSet::new();
    for group in &metadata.groups {
        if !groups.insert(*group) {
            return Err(AppError::Validation(format!(
                "backup metadata declares duplicate group `{}`",
                group.as_str()
            )));
        }
    }

    if !metadata
        .flavor
        .eq_ignore_ascii_case(installation.flavor.as_str())
    {
        return Err(AppError::Validation(format!(
            "backup flavor `{}` does not match target flavor `{}`",
            metadata.flavor,
            installation.flavor.as_str()
        )));
    }

    Ok(())
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

fn parse_backup_entry_target(
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

pub(super) fn reject_unsupported_backup_source_symlink(
    source_kind: &str,
    entry_path: &Path,
    is_symlink: bool,
) -> AppResult<()> {
    reject_unsupported_symlink_metadata(
        &format!("backup {source_kind} entry"),
        &entry_path.display().to_string(),
        is_symlink,
    )
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
    current <= 3 || current == total || total <= 25 || current % 25 == 0
}

fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::core::archive_io::PortableArchivePathSet;
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

    use super::{BackupGroup, parse_backup_entry_target, register_backup_archive_output};

    #[test]
    fn register_backup_archive_output_rejects_case_insensitive_metadata_collisions() {
        let mut archive_outputs = PortableArchivePathSet::new();
        register_backup_archive_output(&mut archive_outputs, "backup.toml", false)
            .expect("backup metadata should register");

        let error = register_backup_archive_output(&mut archive_outputs, "BACKUP.toml", false)
            .expect_err("case-only metadata collision should fail");

        let message = error.to_string();
        assert!(message.contains("case-insensitive archive path collisions"));
        assert!(message.contains("backup.toml"));
        assert!(message.contains("BACKUP.toml"));
    }

    #[test]
    fn register_backup_archive_output_rejects_file_as_ancestor_conflicts() {
        let mut archive_outputs = PortableArchivePathSet::new();
        register_backup_archive_output(&mut archive_outputs, "addons/WeakAuras", false)
            .expect("file output should register");

        let error = register_backup_archive_output(
            &mut archive_outputs,
            "addons/WeakAuras/Config.lua",
            false,
        )
        .expect_err("file ancestor conflict should fail");

        let message = error.to_string();
        assert!(message.contains("conflicting file and directory archive paths"));
        assert!(message.contains("addons/WeakAuras"));
        assert!(message.contains("addons/WeakAuras/Config.lua"));
    }

    #[test]
    fn register_backup_archive_output_allows_directory_ancestors() {
        let mut archive_outputs = PortableArchivePathSet::new();
        register_backup_archive_output(&mut archive_outputs, "addons/WeakAuras", true)
            .expect("directory output should register");
        register_backup_archive_output(&mut archive_outputs, "addons/WeakAuras/Config.lua", false)
            .expect("directory ancestors should stay legal");
    }

    #[test]
    fn parse_backup_entry_target_maps_group_and_destination() {
        let installation = fixture_installation();

        let target = parse_backup_entry_target(
            "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua",
            &installation,
        )
        .expect("portable backup entry should parse");

        assert_eq!(target.group, BackupGroup::Wtf);
        assert_eq!(
            target.destination,
            installation
                .wtf_dir
                .join("common")
                .join("accounts")
                .join("ACCOUNT")
                .join("SavedVariables")
                .join("Details.lua")
        );
    }

    #[test]
    fn parse_backup_entry_target_preserves_root_vs_path_errors() {
        let installation = fixture_installation();

        let missing_rest = parse_backup_entry_target("addons", &installation)
            .expect_err("root-only entry path should fail");
        assert!(
            missing_rest
                .to_string()
                .contains("backup archive contains unsupported entry path")
        );

        let unsupported_root = parse_backup_entry_target("metadata/backup.toml", &installation)
            .expect_err("unsupported root should fail");
        assert!(
            unsupported_root
                .to_string()
                .contains("backup archive contains unsupported root entry")
        );
    }

    fn fixture_installation() -> DetectedFlavorInstallation {
        let product_root = PathBuf::from("C:/Games/World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root,
            flavor_root,
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        }
    }
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
pub(super) fn set_restore_test_failure_after(limit: Option<usize>) {
    TEST_RESTORE_FAIL_AFTER.with(|fail_after| fail_after.set(limit));
}
