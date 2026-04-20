use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use super::super::*;
use crate::core::lua_patch::rewrite_lua_file;

pub(in crate::core::bundle) fn execute_apply_operations<TBeforeOperation>(
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
    let mut source_reader = source.open_reader()?;
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
            &mut source_reader,
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

fn materialize_operation_source(
    operation_index: usize,
    operation: &PreparedApplyOperation,
    source: &PreparedApplySource,
    source_reader: &mut ApplySourceReader,
    stage_root: &Path,
) -> AppResult<PathBuf> {
    let staged_path = staged_operation_path(operation_index, &operation.archive_name, stage_root);
    source.materialize_logical_entry(source_reader, &operation.archive_name, &staged_path)?;
    Ok(staged_path)
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
