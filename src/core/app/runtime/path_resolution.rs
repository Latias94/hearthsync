use std::path::{Path, PathBuf};

use crate::core::error::{AppError, AppResult};

pub(super) fn validate_relative_path_base(
    relative_path_base: Option<PathBuf>,
) -> AppResult<Option<PathBuf>> {
    if let Some(base) = relative_path_base.as_deref()
        && !base.is_absolute()
    {
        return Err(AppError::Validation(format!(
            "app runtime relative path base must be absolute: {}",
            base.display()
        )));
    }

    Ok(relative_path_base)
}

pub(super) fn resolve_optional_runtime_paths(
    paths: Option<Vec<PathBuf>>,
    base: Option<&Path>,
    description: &str,
) -> AppResult<Option<Vec<PathBuf>>> {
    paths
        .map(|paths| {
            paths
                .into_iter()
                .map(|path| resolve_runtime_path(path, base, description))
                .collect()
        })
        .transpose()
}

pub(super) fn resolve_optional_runtime_path(
    path: Option<PathBuf>,
    base: Option<&Path>,
    description: &str,
) -> AppResult<Option<PathBuf>> {
    path.map(|path| resolve_runtime_path(path, base, description))
        .transpose()
}

pub(super) fn resolve_runtime_path(
    path: PathBuf,
    base: Option<&Path>,
    description: &str,
) -> AppResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    let Some(base) = base else {
        return Err(AppError::Validation(format!(
            "{description} relative path requires an app runtime relative path base: {}",
            path.display()
        )));
    };
    if !base.is_absolute() {
        return Err(AppError::Validation(format!(
            "app runtime relative path base must be absolute before resolving {description}: {}",
            base.display()
        )));
    }

    Ok(base.join(path))
}
