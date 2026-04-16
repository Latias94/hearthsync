use std::fs;
use std::path::{Path, PathBuf};

use crate::core::error::AppResult;

use super::TrackedAddon;

pub(super) fn inspect_staged_addon(stage_path: &Path, addon_name: &str) -> AppResult<TrackedAddon> {
    let toc_path = find_primary_toc(stage_path, addon_name)?;
    let (toc_file, title, version) = if let Some(path) = toc_path {
        let toc_file = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string());
        let content = fs::read_to_string(&path).unwrap_or_default();
        let title = extract_toc_field(&content, "Title");
        let version = extract_toc_field(&content, "Version");
        (toc_file, title, version)
    } else {
        (None, None, None)
    };

    Ok(TrackedAddon {
        directory_name: addon_name.to_string(),
        toc_file,
        title,
        version,
    })
}

pub(crate) fn find_primary_toc(stage_path: &Path, addon_name: &str) -> AppResult<Option<PathBuf>> {
    if !stage_path.exists() {
        return Ok(None);
    }

    let preferred = stage_path.join(format!("{addon_name}.toc"));
    if preferred.exists() {
        return Ok(Some(preferred));
    }

    for entry in fs::read_dir(stage_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toc"))
        {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn extract_toc_field(content: &str, field: &str) -> Option<String> {
    let needle = format!("## {field}:");
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&needle)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}
