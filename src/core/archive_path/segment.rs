use std::path::{Component, Path};

use crate::core::error::{AppError, AppResult};

pub(in crate::core) fn validate_portable_path_segment(
    segment: &str,
    segment_kind: &str,
) -> AppResult<()> {
    if !is_safe_archive_segment(segment) {
        return Err(AppError::Validation(format!(
            "invalid {segment_kind} name: `{segment}`"
        )));
    }

    Ok(())
}

pub(in crate::core) fn safe_zip_segments(archive_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in archive_name.split('/') {
        if !is_safe_archive_segment(segment) {
            return Err(AppError::Validation(format!(
                "unsafe archive path: `{archive_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

pub(in crate::core) fn safe_zip_segments_under<'a>(
    path: &'a str,
    root_segment: &str,
    path_kind: &str,
) -> AppResult<Vec<&'a str>> {
    let segments = safe_zip_segments(path)
        .map_err(|_| AppError::Validation(format!("unsafe {path_kind}: `{path}`")))?;

    if segments.first().copied() != Some(root_segment) || segments.len() < 2 {
        return Err(AppError::Validation(format!(
            "{path_kind} must be under `{root_segment}/`: {path}"
        )));
    }

    Ok(segments)
}

pub(in crate::core) fn safe_relative_segments(
    relative_path: &Path,
    path_kind: &str,
) -> AppResult<Vec<String>> {
    let mut segments = Vec::new();

    for component in relative_path.components() {
        let Component::Normal(segment) = component else {
            return Err(AppError::Validation(format!(
                "unsafe {path_kind}: {}",
                relative_path.display()
            )));
        };

        let segment = segment.to_string_lossy().to_string();
        if !is_safe_archive_segment(&segment) {
            return Err(AppError::Validation(format!(
                "unsafe {path_kind}: {}",
                relative_path.display()
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

fn is_safe_archive_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.ends_with([' ', '.'])
        && !segment.chars().any(|char| {
            char.is_control()
                || matches!(char, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '/' | '\\')
        })
        && !is_windows_reserved_device_name(segment)
}

fn is_windows_reserved_device_name(segment: &str) -> bool {
    let stem = segment
        .split('.')
        .next()
        .unwrap_or(segment)
        .trim_end_matches([' ', '.']);
    let upper = stem.to_ascii_uppercase();

    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || is_numbered_windows_device_name(&upper, "COM")
        || is_numbered_windows_device_name(&upper, "LPT")
}

fn is_numbered_windows_device_name(value: &str, prefix: &str) -> bool {
    let Some(suffix) = value.strip_prefix(prefix) else {
        return false;
    };

    suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9')
}
