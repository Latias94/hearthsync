use std::path::{Path, PathBuf};

use crate::core::error::{AppError, AppResult};

pub(in crate::core::bundle) fn validate_plain_name(kind: &str, value: &str) -> AppResult<()> {
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

pub(in crate::core::bundle) fn safe_zip_segments(archive_name: &str) -> AppResult<Vec<&str>> {
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

fn is_safe_archive_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.ends_with([' ', '.'])
        && !segment.chars().any(|char| {
            char.is_control() || matches!(char, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '\\')
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

pub(in crate::core::bundle) fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

pub(in crate::core::bundle) fn resolve_zip_style_path(
    root: &Path,
    archive_name: &str,
) -> AppResult<PathBuf> {
    let segments = safe_zip_segments(archive_name)?;
    Ok(join_segments(root, &segments))
}

pub(in crate::core::bundle) fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case(".DS_Store")
                || name.eq_ignore_ascii_case("Thumbs.db")
                || name.eq_ignore_ascii_case("desktop.ini")
        })
}

pub(in crate::core::bundle) fn safe_file_part(value: &str) -> String {
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

pub(in crate::core::bundle) fn to_zip_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::safe_zip_segments;

    #[test]
    fn safe_zip_segments_rejects_windows_reserved_characters() {
        let error = safe_zip_segments("addons/Weak:Auras/WeakAuras.toc")
            .expect_err("colon should be rejected");

        assert!(error.to_string().contains("unsafe archive path"));
    }

    #[test]
    fn safe_zip_segments_rejects_windows_device_names() {
        for archive_name in [
            "addons/CON/Config.lua",
            "addons/aux.lua",
            "addons/com1.txt",
            "addons/LPT9",
        ] {
            let error = safe_zip_segments(archive_name).expect_err("device name should fail");
            assert!(error.to_string().contains("unsafe archive path"));
        }
    }

    #[test]
    fn safe_zip_segments_rejects_trailing_space_or_dot() {
        for archive_name in ["addons/WeakAuras /Core.lua", "addons/WeakAuras./Core.lua"] {
            let error =
                safe_zip_segments(archive_name).expect_err("trailing space or dot should fail");
            assert!(error.to_string().contains("unsafe archive path"));
        }
    }

    #[test]
    fn safe_zip_segments_accepts_portable_names() {
        assert_eq!(
            safe_zip_segments("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua")
                .expect("portable path"),
            vec![
                "wtf",
                "common",
                "accounts",
                "ACCOUNT",
                "SavedVariables",
                "Details.lua"
            ]
        );
    }
}
