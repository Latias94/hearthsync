use std::path::Path;

use super::super::*;

pub(in crate::core::bundle) fn rollback_or_report_apply_error<T>(
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
