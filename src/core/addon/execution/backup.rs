use std::path::PathBuf;

use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
pub(super) fn create_addon_backup(
    installation: &DetectedFlavorInstallation,
    output_path: Option<PathBuf>,
    label: &str,
) -> AppResult<PathBuf> {
    Ok(create_backup(BackupRequest {
        installation: installation.clone(),
        output_path,
        groups: vec![BackupGroup::Addons],
        label: Some(label.to_string()),
    })?
    .archive_path)
}
