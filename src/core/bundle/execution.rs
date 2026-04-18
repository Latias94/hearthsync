use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::ZipArchive;

use super::archive_read::extract_archive_entry_to_path;
use super::*;
use crate::core::lua_patch::rewrite_lua_file;

pub(super) fn execute_apply_operations<TBeforeOperation>(
    source: &PreparedApplySource,
    execution_operations: &[PreparedApplyOperation],
    manifest: &BundleManifest,
    mut before_operation: TBeforeOperation,
) -> AppResult<(usize, usize)>
where
    TBeforeOperation: FnMut(usize, usize, &PreparedApplyOperation) -> AppResult<()>,
{
    let mut written_files = 0usize;
    let mut rewritten_files = 0usize;
    let rewrite_stage = tempdir()?;
    let mut zip_source = open_zip_source(source)?;
    let rewrite_options = LuaRewriteOptions {
        rewrite_profile_keys: manifest.mapping.rewrite_profile_keys,
        rewrite_identity_strings: manifest.mapping.rewrite_identity_strings,
    };

    for (operation_index, operation) in execution_operations.iter().enumerate() {
        before_operation(operation_index, execution_operations.len(), operation)?;

        if matches!(operation.action, ApplyAction::Skip | ApplyAction::Preserve) {
            continue;
        }

        if operation.action == ApplyAction::Remove {
            remove_target_path(&operation.destination)?;
            continue;
        }

        if let Some(parent) = operation.destination.parent() {
            fs::create_dir_all(parent)?;
        }

        let source_path = materialize_operation_source(
            operation_index,
            operation,
            source,
            zip_source.as_mut(),
            rewrite_stage.path(),
        )?;
        let rewrite_applied = if operation.rewrites.is_empty() {
            false
        } else {
            rewrite_lua_file(
                Path::new(&operation.archive_name),
                &source_path,
                &operation.rewrites,
                rewrite_options,
            )?
        };

        fs::copy(&source_path, &operation.destination)?;
        written_files += 1;

        if rewrite_applied {
            rewritten_files += 1;
        }
    }

    Ok((written_files, rewritten_files))
}

fn open_zip_source(source: &PreparedApplySource) -> AppResult<Option<ZipArchive<File>>> {
    match source {
        PreparedApplySource::BundleArchive { bundle_path } => {
            let file = File::open(bundle_path)?;
            Ok(Some(ZipArchive::new(file)?))
        }
        PreparedApplySource::ExternalPackage {
            source_path,
            source_kind: ExternalPackageSourceKind::ZipArchive,
            ..
        } => {
            let file = File::open(source_path)?;
            Ok(Some(ZipArchive::new(file)?))
        }
        PreparedApplySource::ExternalPackage {
            source_kind: ExternalPackageSourceKind::Directory,
            ..
        } => Ok(None),
    }
}

fn materialize_operation_source(
    operation_index: usize,
    operation: &PreparedApplyOperation,
    source: &PreparedApplySource,
    zip_source: Option<&mut ZipArchive<File>>,
    stage_root: &Path,
) -> AppResult<PathBuf> {
    let staged_path = staged_operation_path(operation_index, &operation.archive_name, stage_root);

    match source {
        PreparedApplySource::BundleArchive { .. } => {
            let archive = zip_source.ok_or_else(|| {
                AppError::Validation(
                    "bundle apply expected an open archive source during execution".to_string(),
                )
            })?;
            extract_archive_entry_to_path(archive, &operation.archive_name, &staged_path)?;
        }
        PreparedApplySource::ExternalPackage {
            source_path,
            source_kind: ExternalPackageSourceKind::Directory,
            entry_source_map,
        } => {
            let entry_path = lookup_external_package_entry_source_path(
                entry_source_map,
                &operation.archive_name,
            )?;
            let resolved_path = resolve_zip_style_path(source_path, entry_path)?;
            if let Some(parent) = staged_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(resolved_path, &staged_path)?;
        }
        PreparedApplySource::ExternalPackage {
            source_kind: ExternalPackageSourceKind::ZipArchive,
            entry_source_map,
            ..
        } => {
            let entry_path = lookup_external_package_entry_source_path(
                entry_source_map,
                &operation.archive_name,
            )?;
            let archive = zip_source.ok_or_else(|| {
                AppError::Validation(
                    "external package apply expected an open archive source during execution"
                        .to_string(),
                )
            })?;
            extract_archive_entry_to_path(archive, entry_path, &staged_path)?;
        }
    }

    Ok(staged_path)
}

fn lookup_external_package_entry_source_path<'a>(
    entry_source_map: &'a std::collections::BTreeMap<String, String>,
    archive_name: &str,
) -> AppResult<&'a str> {
    entry_source_map
        .get(archive_name)
        .map(String::as_str)
        .ok_or_else(|| {
            AppError::Validation(format!(
                "external-package apply operation is missing a source path: {archive_name}"
            ))
        })
}

fn staged_operation_path(operation_index: usize, archive_name: &str, stage_root: &Path) -> PathBuf {
    let file_name = Path::new(archive_name)
        .file_name()
        .map(|name| name.to_owned())
        .unwrap_or_else(|| format!("operation-{operation_index}").into());
    stage_root.join(operation_index.to_string()).join(file_name)
}

fn remove_target_path(path: &Path) -> AppResult<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub(super) fn rollback_or_report_apply_error<T>(
    error: AppError,
    backup_path: Option<&Path>,
    installation: &DetectedFlavorInstallation,
    operation_name: &str,
) -> AppResult<T> {
    let Some(backup_path) = backup_path else {
        return Err(error);
    };

    match restore_backup(backup_path, installation) {
        Ok(restored) => match error {
            AppError::Cancelled(message) => Err(AppError::Cancelled(format!(
                "{message}; rollback restored `{}` ({} files)",
                restored.archive_path.display(),
                restored.restored_files
            ))),
            other => Err(AppError::Validation(format!(
                "{operation_name} failed and rollback restored `{}` ({} files): {other}",
                restored.archive_path.display(),
                restored.restored_files
            ))),
        },
        Err(rollback_error) => Err(AppError::Validation(format!(
            "{operation_name} failed: {error}; rollback failed: {rollback_error}"
        ))),
    }
}

pub(super) fn file_contents_equal_to_bytes(bytes: &[u8], right: &Path) -> AppResult<bool> {
    if !right.exists() || !right.is_file() {
        return Ok(false);
    }

    let right_metadata = fs::metadata(right)?;
    if right_metadata.len() != bytes.len() as u64 {
        return Ok(false);
    }

    let mut right_file = File::open(right)?;
    let mut right_buffer = [0u8; 8192];
    let mut offset = 0usize;

    loop {
        let right_read = right_file.read(&mut right_buffer)?;
        if right_read == 0 {
            return Ok(offset == bytes.len());
        }
        if offset + right_read > bytes.len() {
            return Ok(false);
        }
        if bytes[offset..offset + right_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        offset += right_read;
    }
}
