use std::fs::{self, File};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::core::error::{AppError, AppResult};

use super::PreparedAddonDirectory;
use super::inspect::inspect_staged_addon;

pub(super) fn extract_archive_addons(
    archive_path: &Path,
    stage_root: &Path,
) -> AppResult<Vec<PreparedAddonDirectory>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let addon_roots = discover_archive_addon_roots(&mut archive)?;
    if addon_roots.is_empty() {
        return Ok(Vec::new());
    }

    let mut file_counts = vec![0usize; addon_roots.len()];

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?;
        if segments.is_empty() {
            continue;
        }

        let Some(root_index) = match_addon_root(&segments, &addon_roots) else {
            continue;
        };
        let root = &addon_roots[root_index];
        let addon_name = root
            .last()
            .map(String::as_str)
            .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))?;
        let relative = &segments[root.len()..];
        let destination =
            join_segments(stage_root, &[addon_name]).join(join_segments(Path::new(""), relative));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(destination)?;
        std::io::copy(&mut entry, &mut output)?;
        file_counts[root_index] += 1;
    }

    let mut prepared = Vec::new();
    for (index, root) in addon_roots.iter().enumerate() {
        let addon_name = root
            .last()
            .map(String::as_str)
            .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))?;
        let stage_path = stage_root.join(addon_name);
        let addon = inspect_staged_addon(&stage_path, addon_name)?;
        prepared.push(PreparedAddonDirectory {
            addon,
            stage_path,
            file_count: file_counts[index],
        });
    }

    prepared.sort_by(|left, right| left.addon.directory_name.cmp(&right.addon.directory_name));
    Ok(prepared)
}

fn discover_archive_addon_roots(archive: &mut ZipArchive<File>) -> AppResult<Vec<Vec<String>>> {
    let mut roots = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?;
        if segments.len() < 2 {
            continue;
        }

        let file_name = segments
            .last()
            .copied()
            .ok_or_else(|| AppError::Validation("invalid archive entry".to_string()))?;
        if !file_name.ends_with(".toc") {
            continue;
        }

        let Some(file_stem) = Path::new(file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
        else {
            continue;
        };
        let parent_name = segments[segments.len() - 2];
        if file_stem != parent_name {
            continue;
        }

        let root = segments[..segments.len() - 1]
            .iter()
            .map(|segment| (*segment).to_string())
            .collect::<Vec<_>>();
        if !roots.contains(&root) {
            roots.push(root);
        }
    }

    roots.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    Ok(roots)
}

fn match_addon_root(segments: &[&str], roots: &[Vec<String>]) -> Option<usize> {
    roots.iter().position(|root| {
        root.len() <= segments.len()
            && root
                .iter()
                .zip(segments.iter())
                .all(|(left, right)| left.as_str() == *right)
    })
}

fn safe_zip_segments(entry_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in entry_name.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe archive path: `{entry_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}
