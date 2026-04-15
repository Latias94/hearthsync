use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct BundleAddonSourceIndex {
    pub(super) schema_version: u32,
    pub(super) sources: Vec<BundleAddonSourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct BundleAddonSourceEntry {
    pub(super) comparison_key: String,
    pub(super) package_id: String,
    pub(super) path: String,
    pub(super) content_sha256: String,
    pub(super) addon_directories: Vec<String>,
}

pub(super) fn validate_plain_name(kind: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
    {
        return Err(AppError::Validation(format!(
            "invalid {kind} name: `{value}`"
        )));
    }

    Ok(())
}

pub(super) fn safe_zip_segments(archive_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in archive_name.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe archive path: `{archive_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

pub(super) fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

pub(super) fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case(".DS_Store")
                || name.eq_ignore_ascii_case("Thumbs.db")
                || name.eq_ignore_ascii_case("desktop.ini")
        })
}

pub(super) fn safe_file_part(value: &str) -> String {
    let mut output = value
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() || char == '-' || char == '_' {
                char
            } else {
                '-'
            }
        })
        .collect::<String>();

    while output.contains("--") {
        output = output.replace("--", "-");
    }

    output.trim_matches('-').to_string()
}

pub(super) fn to_zip_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub(super) fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

pub(super) fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
