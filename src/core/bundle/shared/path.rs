use std::path::{Path, PathBuf};

pub(in crate::core::bundle) use crate::core::archive_path::{
    join_segments, safe_zip_segments, to_zip_path,
};
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
