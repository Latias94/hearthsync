use std::fs::File;
use std::path::{Component, Path};

use walkdir::WalkDir;
use zip::ZipArchive;

use super::source_entry::SourceEntry;
use super::types::ExternalPackageSourceKind;
use crate::core::bundle::shared::{safe_zip_segments, should_skip_path, to_zip_path};
use crate::core::error::{AppError, AppResult};

pub(super) fn detect_source_kind(path: &Path) -> AppResult<ExternalPackageSourceKind> {
    if path.is_dir() {
        return Ok(ExternalPackageSourceKind::Directory);
    }

    let file = File::open(path)?;
    ZipArchive::new(file).map_err(|error| {
        AppError::Validation(format!(
            "external package source is not a valid zip archive: {} ({error})",
            path.display()
        ))
    })?;
    Ok(ExternalPackageSourceKind::ZipArchive)
}

pub(super) fn collect_source_entries(
    source_path: &Path,
    source_kind: ExternalPackageSourceKind,
) -> AppResult<Vec<SourceEntry>> {
    let mut entries = match source_kind {
        ExternalPackageSourceKind::Directory => collect_directory_entries(source_path)?,
        ExternalPackageSourceKind::ZipArchive => collect_zip_entries(source_path)?,
    };
    entries.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(entries)
}

fn collect_directory_entries(root: &Path) -> AppResult<Vec<SourceEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        if entry.file_type().is_dir() {
            continue;
        }

        if should_skip_path(entry.path()) {
            continue;
        }

        let relative_path = entry.path().strip_prefix(root).map_err(|_| {
            AppError::Validation(format!(
                "failed to derive relative path for external package entry: {}",
                entry.path().display()
            ))
        })?;
        let segments = safe_relative_segments(relative_path)?;
        if should_ignore_source_segments(&segments) {
            continue;
        }

        entries.push(SourceEntry {
            source_path: to_zip_path(relative_path),
            segments,
        });
    }

    Ok(entries)
}

fn collect_zip_entries(path: &Path) -> AppResult<Vec<SourceEntry>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entries = Vec::new();

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
        if should_ignore_source_segments(&segments) {
            continue;
        }

        if Path::new(&entry_name)
            .file_name()
            .is_some_and(|name| should_skip_path(Path::new(name)))
        {
            continue;
        }

        entries.push(SourceEntry {
            source_path: entry_name,
            segments,
        });
    }

    Ok(entries)
}

fn safe_relative_segments(relative_path: &Path) -> AppResult<Vec<String>> {
    let mut segments = Vec::new();

    for component in relative_path.components() {
        let Component::Normal(segment) = component else {
            return Err(AppError::Validation(format!(
                "unsafe directory entry path: {}",
                relative_path.display()
            )));
        };
        let segment = segment.to_string_lossy().to_string();
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(AppError::Validation(format!(
                "unsafe directory entry path: {}",
                relative_path.display()
            )));
        }
        segments.push(segment);
    }

    Ok(segments)
}

fn should_ignore_source_segments(segments: &[String]) -> bool {
    segments
        .iter()
        .any(|segment| segment.eq_ignore_ascii_case("__MACOSX"))
}
