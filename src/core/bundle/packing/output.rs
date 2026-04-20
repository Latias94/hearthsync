use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::super::safe_file_part;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::manifest::BundleManifest;

pub(super) fn resolve_bundle_output_path(
    output_path: Option<&Path>,
    manifest: &BundleManifest,
    timestamp: &str,
    default_base_dir: &Path,
) -> AppResult<PathBuf> {
    let file_name = format!(
        "bundle-{}-{}.zip",
        safe_file_part(&manifest.package.id),
        compact_timestamp(timestamp)
    );

    match output_path {
        Some(path) if path.extension().is_some_and(|extension| extension == "zip") => {
            Ok(resolve_output_reference(path, default_base_dir))
        }
        Some(path) => Ok(resolve_output_reference(path, default_base_dir).join(file_name)),
        None => Ok(default_base_dir.join(file_name)),
    }
}

pub(super) fn default_bundle_output_base_dir(
    installation: &DetectedFlavorInstallation,
    manifest_base_dir: Option<&Path>,
) -> PathBuf {
    manifest_base_dir
        .map(Path::to_path_buf)
        .or_else(|| installation.product_root.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| installation.product_root.clone())
}

fn resolve_output_reference(path: &Path, default_base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        default_base_dir.join(path)
    }
}

pub(super) fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn compact_timestamp(timestamp: &str) -> String {
    timestamp
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect::<String>()
}
