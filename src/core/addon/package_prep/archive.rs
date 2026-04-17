use std::fs::File;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::core::addon_layout::discover_addon_roots_from_entry_segments;
use crate::core::archive_io::copy_reader_to_path;
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
        copy_reader_to_path(&mut entry, &destination)?;
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
    let mut entry_segments = Vec::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?
            .into_iter()
            .map(|segment| segment.to_string())
            .collect::<Vec<_>>();
        if segments.is_empty() {
            continue;
        }

        entry_segments.push(segments);
    }

    Ok(discover_addon_roots_from_entry_segments(
        entry_segments.iter().map(|segments| segments.as_slice()),
    ))
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
