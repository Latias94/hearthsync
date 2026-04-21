use std::path::{Path, PathBuf};

use crate::core::archive_path::validate_portable_path_segment;
pub(in crate::core::bundle) use crate::core::archive_path::{
    join_segments, safe_zip_segments, to_zip_path,
};
use crate::core::error::AppResult;

pub(in crate::core::bundle) fn validate_plain_name(kind: &str, value: &str) -> AppResult<()> {
    validate_portable_path_segment(value, kind)
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

#[cfg(test)]
mod tests {
    use super::validate_plain_name;

    #[test]
    fn validate_plain_name_rejects_non_portable_segments() {
        let error = validate_plain_name("account", "CON")
            .expect_err("reserved Windows device name should fail");

        assert!(error.to_string().contains("invalid account name"));
    }

    #[test]
    fn validate_plain_name_rejects_trailing_dot_or_space() {
        for value in ["WeakAuras.", "WeakAuras "] {
            let error =
                validate_plain_name("addon", value).expect_err("trailing dot or space should fail");

            assert!(error.to_string().contains("invalid addon name"));
        }
    }

    #[test]
    fn validate_plain_name_accepts_portable_names() {
        validate_plain_name("target account", "Account#1").expect("portable plain name");
    }
}
