use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::core::error::{AppError, AppResult};

const APP_NAME: &str = "hearthsync";

pub(crate) fn app_data_subdir(relative_path: &Path) -> AppResult<PathBuf> {
    ProjectDirs::from("", "", APP_NAME)
        .map(|dirs| dirs.data_local_dir().join(relative_path))
        .ok_or_else(|| {
            AppError::Validation(
                "failed to determine platform-specific app data directory".to_string(),
            )
        })
}
