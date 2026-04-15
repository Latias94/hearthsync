use std::fs::File;
use std::io::Read;

use tempfile::tempdir;
use zip::ZipArchive;

use super::archive_io::extract_archive_entry_to_path;
use super::*;
use crate::core::lua_patch::rewrite_lua_file;

pub(super) fn execute_apply_operations(
    bundle_path: &Path,
    execution_operations: &[PreparedApplyOperation],
    manifest: &BundleManifest,
) -> AppResult<(usize, usize)> {
    let mut written_files = 0usize;
    let mut rewritten_files = 0usize;
    let rewrite_stage = tempdir()?;
    let file = File::open(bundle_path)?;
    let mut archive = ZipArchive::new(file)?;
    let rewrite_options = LuaRewriteOptions {
        rewrite_profile_keys: manifest.mapping.rewrite_profile_keys,
        rewrite_identity_strings: manifest.mapping.rewrite_identity_strings,
    };

    for (operation_index, operation) in execution_operations.iter().enumerate() {
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
        let source_path = if operation.rewrite_applied {
            materialize_rewritten_operation(
                operation_index,
                operation,
                &mut archive,
                rewrite_stage.path(),
                rewrite_options,
            )?
        } else {
            materialize_archive_operation(
                operation_index,
                &operation.archive_name,
                &mut archive,
                rewrite_stage.path(),
            )?
        };
        fs::copy(source_path, &operation.destination)?;
        written_files += 1;

        if operation.rewrite_applied {
            rewritten_files += 1;
        }
    }

    Ok((written_files, rewritten_files))
}

fn materialize_rewritten_operation(
    operation_index: usize,
    operation: &PreparedApplyOperation,
    archive: &mut ZipArchive<File>,
    rewrite_stage_root: &Path,
    rewrite_options: LuaRewriteOptions,
) -> AppResult<PathBuf> {
    let rewrite_path = materialize_archive_operation(
        operation_index,
        &operation.archive_name,
        archive,
        rewrite_stage_root,
    )?;
    rewrite_lua_file(
        Path::new(&operation.archive_name),
        &rewrite_path,
        &operation.rewrites,
        rewrite_options,
    )?;
    Ok(rewrite_path)
}

fn materialize_archive_operation(
    operation_index: usize,
    archive_name: &str,
    archive: &mut ZipArchive<File>,
    stage_root: &Path,
) -> AppResult<PathBuf> {
    let file_name = Path::new(archive_name)
        .file_name()
        .map(|name| name.to_owned())
        .unwrap_or_else(|| format!("operation-{operation_index}").into());
    let stage_path = stage_root.join(operation_index.to_string()).join(file_name);
    extract_archive_entry_to_path(archive, archive_name, &stage_path)?;
    Ok(stage_path)
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
) -> AppResult<T> {
    let Some(backup_path) = backup_path else {
        return Err(error);
    };

    match restore_backup(backup_path, installation) {
        Ok(restored) => Err(AppError::Validation(format!(
            "bundle apply failed and rollback restored `{}` ({} files): {error}",
            restored.archive_path.display(),
            restored.restored_files
        ))),
        Err(rollback_error) => Err(AppError::Validation(format!(
            "bundle apply failed: {error}; rollback failed: {rollback_error}"
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
