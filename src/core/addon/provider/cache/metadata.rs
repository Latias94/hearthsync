use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::super::validation::{RemoteArchiveValidators, file_sha256};
use super::super::{AddonProviderOptions, AddonSourceRef};
use super::download::normalize_archive_name;
use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::{AppError, AppResult};

pub(super) const CACHE_METADATA_SUFFIX: &str = ".hearthsync-cache.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(in crate::core::addon::provider) struct CachedArchiveMetadata {
    pub(in crate::core::addon::provider) source_display_name: String,
    #[serde(default)]
    pub(in crate::core::addon::provider) source_ref: Option<AddonSourceRef>,
    pub(in crate::core::addon::provider) archive_name: String,
    pub(in crate::core::addon::provider) file_size: u64,
    pub(in crate::core::addon::provider) file_sha256: String,
    #[serde(default)]
    pub(in crate::core::addon::provider) fetched_at_unix_timestamp: Option<u64>,
    #[serde(default)]
    pub(in crate::core::addon::provider) remote_validators: RemoteArchiveValidators,
}

pub(in crate::core::addon::provider) fn cached_archive_metadata_path(
    archive_path: &Path,
) -> PathBuf {
    let file_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("downloaded-addon.zip");
    archive_path.with_file_name(format!("{file_name}{CACHE_METADATA_SUFFIX}"))
}

pub(in crate::core::addon::provider) fn write_cached_archive_metadata(
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
    remote_validators: &RemoteArchiveValidators,
    options: &AddonProviderOptions,
) -> AppResult<()> {
    if options.download_cache_dir.is_none() {
        return Ok(());
    }

    let metadata = CachedArchiveMetadata {
        source_display_name: source.display_name(),
        source_ref: Some(source.clone()),
        archive_name: normalize_archive_name(archive_name),
        file_size: fs::metadata(archive_path)?.len(),
        file_sha256: file_sha256(archive_path)?,
        fetched_at_unix_timestamp: Some(current_unix_timestamp_secs()?),
        remote_validators: remote_validators.clone(),
    };
    let metadata_bytes = serde_json::to_vec_pretty(&metadata)?;
    write_bytes_atomically(&cached_archive_metadata_path(archive_path), &metadata_bytes)
}

pub(super) fn current_unix_timestamp_secs() -> AppResult<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| AppError::Validation("system clock is before unix epoch".to_string()))
}

pub(in crate::core::addon::provider) fn cached_archive_metadata_if_local_file_matches(
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
) -> Option<CachedArchiveMetadata> {
    let metadata = load_cached_archive_metadata(archive_path)?;
    cached_archive_metadata_matches_local_file(&metadata, archive_path, source, archive_name)
        .then_some(metadata)
}

pub(super) fn cached_archive_matches_metadata(
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
    remote_validators: &RemoteArchiveValidators,
) -> bool {
    let Some(metadata) =
        cached_archive_metadata_if_local_file_matches(archive_path, source, archive_name)
    else {
        return false;
    };

    if remote_validators.is_empty() {
        return true;
    }

    metadata.remote_validators == *remote_validators
}

pub(super) fn archive_path_from_metadata_sidecar(metadata_path: &Path) -> Option<PathBuf> {
    let file_name = metadata_path.file_name()?.to_str()?;
    let archive_name = file_name.strip_suffix(CACHE_METADATA_SUFFIX)?;
    Some(metadata_path.with_file_name(archive_name))
}

fn load_cached_archive_metadata(archive_path: &Path) -> Option<CachedArchiveMetadata> {
    let metadata_path = cached_archive_metadata_path(archive_path);
    let metadata_bytes = fs::read(&metadata_path).ok()?;
    serde_json::from_slice::<CachedArchiveMetadata>(&metadata_bytes).ok()
}

fn cached_archive_metadata_matches_local_file(
    metadata: &CachedArchiveMetadata,
    archive_path: &Path,
    source: &AddonSourceRef,
    archive_name: &str,
) -> bool {
    if metadata.source_display_name != source.display_name() {
        return false;
    }
    if metadata.archive_name != normalize_archive_name(archive_name) {
        return false;
    }

    let Ok(file_metadata) = fs::metadata(archive_path) else {
        return false;
    };
    if metadata.file_size != file_metadata.len() {
        return false;
    }

    let Ok(file_sha256) = file_sha256(archive_path) else {
        return false;
    };
    metadata.file_sha256 == file_sha256
}
