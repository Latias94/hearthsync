use std::fs::File;
use std::path::Path;

use walkdir::WalkDir;
use zip::ZipArchive;

use super::source_entry::SourceEntry;
use super::types::{ExternalPackageLayout, ExternalPackageSourceKind};
use crate::core::archive_io::{
    reject_unsupported_symlink_metadata, validated_zip_file_entry_segments,
};
use crate::core::archive_path::{safe_relative_segments, validate_portable_path_segment};
use crate::core::bundle::shared::path::{should_skip_path, to_zip_path};
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
    layout: ExternalPackageLayout,
) -> AppResult<Vec<SourceEntry>> {
    let mut entries = match source_kind {
        ExternalPackageSourceKind::Directory => collect_directory_entries(source_path)?,
        ExternalPackageSourceKind::ZipArchive => collect_zip_entries(source_path, layout)?,
    };
    entries.sort_by(|left, right| left.source_path.cmp(&right.source_path));
    Ok(entries)
}

fn collect_directory_entries(root: &Path) -> AppResult<Vec<SourceEntry>> {
    let mut entries = Vec::new();

    for entry in WalkDir::new(root) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let entry_path = entry.path().display().to_string();
        reject_unsupported_symlink_entry("directory", &entry_path, entry.file_type().is_symlink())?;

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
        let segments = safe_relative_segments(relative_path, "directory entry path")?;
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

fn collect_zip_entries(path: &Path, layout: ExternalPackageLayout) -> AppResult<Vec<SourceEntry>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut entries = Vec::new();

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        let entry_name = entry.name().to_string();
        let Some(segments) = zip_file_entry_segments_for_layout(
            &entry_name,
            entry.is_symlink(),
            entry.is_dir(),
            layout,
        )?
        else {
            continue;
        };
        if should_ignore_source_segments(&segments) {
            continue;
        }

        if segments
            .last()
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

fn zip_file_entry_segments_for_layout(
    entry_name: &str,
    is_symlink: bool,
    is_dir: bool,
    layout: ExternalPackageLayout,
) -> AppResult<Option<Vec<String>>> {
    if !layout_allows_mixed_zip_separators(layout) {
        return validated_zip_file_entry_segments(
            "external package zip entry",
            entry_name,
            is_symlink,
            is_dir,
        );
    }

    reject_unsupported_symlink_metadata("external package zip entry", entry_name, is_symlink)?;
    if is_dir {
        return Ok(None);
    }

    let mut segments = Vec::new();
    for segment in entry_name.split(['/', '\\']) {
        validate_portable_path_segment(segment, "external package zip entry path segment")
            .map_err(|_| AppError::Validation(format!("unsafe archive path: `{entry_name}`")))?;
        segments.push(segment.to_string());
    }

    Ok(Some(segments))
}

fn layout_allows_mixed_zip_separators(layout: ExternalPackageLayout) -> bool {
    matches!(
        layout,
        ExternalPackageLayout::NewBeeBoxAddon
            | ExternalPackageLayout::NewBeeBoxFont
            | ExternalPackageLayout::NewBeeBoxMaterial
            | ExternalPackageLayout::NewBeeBoxWtfAccount
            | ExternalPackageLayout::NewBeeBoxWtfCharacter
    )
}

fn should_ignore_source_segments(segments: &[String]) -> bool {
    segments
        .iter()
        .any(|segment| segment.eq_ignore_ascii_case("__MACOSX"))
}

fn reject_unsupported_symlink_entry(
    source_kind: &str,
    entry_path: &str,
    is_symlink: bool,
) -> AppResult<()> {
    reject_unsupported_symlink_metadata(
        &format!("external package {source_kind} entry"),
        entry_path,
        is_symlink,
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::reject_unsupported_symlink_entry;
    use crate::core::archive_path::safe_relative_segments;

    #[test]
    fn reject_unsupported_symlink_entry_reports_directory_sources() {
        let error =
            reject_unsupported_symlink_entry("directory", "AuthorUI/Fonts/FRIZQT__.ttf", true)
                .expect_err("directory symlink should fail");

        let message = error.to_string();
        assert!(message.contains("external package directory entry"));
        assert!(message.contains("unsupported symlink metadata"));
        assert!(message.contains("AuthorUI/Fonts/FRIZQT__.ttf"));
    }

    #[test]
    fn reject_unsupported_symlink_entry_allows_regular_sources() {
        reject_unsupported_symlink_entry("directory", "AuthorUI/WTF/Config.wtf", false)
            .expect("regular directory entry should pass");
    }

    #[test]
    fn safe_relative_segments_reject_non_portable_directory_segments() {
        for relative_path in [
            Path::new("AuthorUI/Interface/AddOns/Weak:Auras"),
            Path::new("AuthorUI/Fonts/CON.ttf"),
            Path::new("AuthorUI/Fonts/FRIZQT__.ttf "),
        ] {
            let error = safe_relative_segments(relative_path, "directory entry path")
                .expect_err("non-portable directory segment should fail");

            assert!(error.to_string().contains("unsafe directory entry path"));
        }
    }
}
