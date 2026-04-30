use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::download::TEMP_DOWNLOAD_SUFFIX;
use super::metadata::CACHE_METADATA_SUFFIX;
use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonDownloadCachePurgeResult {
    pub cache_dir: Option<PathBuf>,
    pub removed_file_count: usize,
    pub removed_directory_count: usize,
    pub reclaimed_bytes: u64,
}

impl AddonDownloadCachePurgeResult {
    fn not_configured() -> Self {
        Self {
            cache_dir: None,
            removed_file_count: 0,
            removed_directory_count: 0,
            reclaimed_bytes: 0,
        }
    }

    fn for_cache_dir(cache_dir: PathBuf, stats: RemovedPathStats) -> Self {
        Self {
            cache_dir: Some(cache_dir),
            removed_file_count: stats.removed_file_count,
            removed_directory_count: stats.removed_directory_count,
            reclaimed_bytes: stats.reclaimed_bytes,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct RemovedPathStats {
    pub(super) removed_file_count: usize,
    pub(super) removed_directory_count: usize,
    pub(super) reclaimed_bytes: u64,
}

pub(in crate::core::addon::provider) fn purge_download_cache_dir(
    cache_dir: Option<&Path>,
) -> AppResult<AddonDownloadCachePurgeResult> {
    let Some(cache_dir) = cache_dir else {
        return Ok(AddonDownloadCachePurgeResult::not_configured());
    };

    validate_cache_root(cache_dir)?;
    let mut stats = RemovedPathStats::default();
    if !cache_dir.exists() {
        return Ok(AddonDownloadCachePurgeResult::for_cache_dir(
            cache_dir.to_path_buf(),
            stats,
        ));
    }

    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        remove_path_recursively(&entry.path(), &mut stats)?;
    }

    Ok(AddonDownloadCachePurgeResult::for_cache_dir(
        cache_dir.to_path_buf(),
        stats,
    ))
}

pub(super) fn validate_cache_root(cache_dir: &Path) -> AppResult<()> {
    if cache_dir.exists() && !cache_dir.is_dir() {
        return Err(AppError::Validation(format!(
            "configured addon download cache path is not a directory: {}",
            cache_dir.display()
        )));
    }

    Ok(())
}

pub(super) fn cache_file_paths(cache_dir: &Path) -> AppResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(cache_dir).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Io(std::io::Error::other(error)))?;
        if entry.file_type().is_file() {
            files.push(entry.into_path());
        }
    }

    Ok(files)
}

pub(super) fn remove_empty_cache_directories(
    cache_dir: &Path,
    stats: &mut RemovedPathStats,
) -> AppResult<()> {
    let mut directories = WalkDir::new(cache_dir)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));

    for directory in directories {
        if fs::read_dir(&directory)?.next().is_none() {
            fs::remove_dir(&directory)?;
            stats.removed_directory_count += 1;
        }
    }

    Ok(())
}

pub(super) fn remove_path_if_exists(path: &Path, stats: &mut RemovedPathStats) -> AppResult<bool> {
    if !path.exists() {
        return Ok(false);
    }

    remove_path_recursively(path, stats)?;
    Ok(true)
}

pub(super) fn is_cache_metadata_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(CACHE_METADATA_SUFFIX))
}

pub(super) fn is_temporary_download_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(TEMP_DOWNLOAD_SUFFIX))
}

fn remove_path_recursively(path: &Path, stats: &mut RemovedPathStats) -> AppResult<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            remove_path_recursively(&entry.path(), stats)?;
        }
        fs::remove_dir(path)?;
        stats.removed_directory_count += 1;
        return Ok(());
    }

    fs::remove_file(path)?;
    stats.removed_file_count += 1;
    stats.reclaimed_bytes += metadata.len();
    Ok(())
}
