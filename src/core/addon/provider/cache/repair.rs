use std::fs;
use std::path::{Path, PathBuf};

use super::super::curseforge::{
    remote_validators_for_curseforge_file, resolve_curseforge_file_with_client,
};
use super::super::github::{
    fetch_github_release_with_client, remote_validators_for_github_asset,
    select_github_release_asset,
};
use super::super::http::{HttpClient, HttpHeader};
use super::super::parse::{parse_curseforge_source, parse_github_source};
use super::super::tukui::parse_tukui_source;
use super::super::validation::{
    RemoteArchiveValidators, conditional_request_headers_for_transport_validators, file_sha256,
    remote_validators_for_http_headers,
};
use super::super::{AddonProviderOptions, AddonSourceRef};
use super::download::download_to_path_with_headers;
use super::maintenance::{
    RemovedPathStats, cache_file_paths, is_cache_metadata_path, is_temporary_download_path,
    remove_empty_cache_directories, remove_path_if_exists, validate_cache_root,
};
use super::metadata::{
    CachedArchiveMetadata, archive_path_from_metadata_sidecar, cached_archive_metadata_path,
    write_cached_archive_metadata,
};
use super::policy::{
    AddonCacheRepairRemotePolicy, should_reuse_cached_http_archive_without_transport_validators,
};
use crate::core::boundary_validation::is_http_url;
use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonDownloadCacheRepairResult {
    pub cache_dir: Option<PathBuf>,
    pub remote_policy: AddonCacheRepairRemotePolicy,
    pub scanned_metadata_count: usize,
    pub repaired_entry_count: usize,
    pub invalid_metadata_count: usize,
    pub missing_archive_count: usize,
    pub mismatched_archive_count: usize,
    pub orphan_archive_count: usize,
    pub partial_download_count: usize,
    pub remote_verified_entry_count: usize,
    pub remote_refreshed_entry_count: usize,
    pub remote_skipped_entry_count: usize,
    pub remote_check_failed_count: usize,
    pub expired_freshness_entry_count: usize,
    pub removed_file_count: usize,
    pub removed_directory_count: usize,
    pub reclaimed_bytes: u64,
}

impl AddonDownloadCacheRepairResult {
    fn not_configured(remote_policy: AddonCacheRepairRemotePolicy) -> Self {
        Self {
            cache_dir: None,
            remote_policy,
            scanned_metadata_count: 0,
            repaired_entry_count: 0,
            invalid_metadata_count: 0,
            missing_archive_count: 0,
            mismatched_archive_count: 0,
            orphan_archive_count: 0,
            partial_download_count: 0,
            remote_verified_entry_count: 0,
            remote_refreshed_entry_count: 0,
            remote_skipped_entry_count: 0,
            remote_check_failed_count: 0,
            expired_freshness_entry_count: 0,
            removed_file_count: 0,
            removed_directory_count: 0,
            reclaimed_bytes: 0,
        }
    }

    fn for_cache_dir(cache_dir: PathBuf, remote_policy: AddonCacheRepairRemotePolicy) -> Self {
        Self {
            cache_dir: Some(cache_dir),
            ..Self::not_configured(remote_policy)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheRemoteRepairStatus {
    Unchanged,
    Refreshed,
    Expired,
    Failed,
    Skipped,
}

struct GitHubCacheEntryRepair<'a> {
    archive_path: &'a Path,
    metadata: &'a CachedArchiveMetadata,
    source_ref: &'a AddonSourceRef,
    owner: &'a str,
    repo: &'a str,
    tag: &'a str,
    asset_name: &'a str,
    options: &'a AddonProviderOptions,
}

struct CachedArchiveRefresh<'a> {
    archive_path: &'a Path,
    source_ref: &'a AddonSourceRef,
    archive_name: &'a str,
    download_url: &'a str,
    headers: Vec<HttpHeader>,
    remote_validators: &'a RemoteArchiveValidators,
    options: &'a AddonProviderOptions,
}

pub(in crate::core::addon::provider) fn repair_download_cache_dir(
    http_client: &impl HttpClient,
    cache_dir: Option<&Path>,
    options: &AddonProviderOptions,
) -> AppResult<AddonDownloadCacheRepairResult> {
    let Some(cache_dir) = cache_dir else {
        return Ok(AddonDownloadCacheRepairResult::not_configured(
            options.cache_repair_remote_policy,
        ));
    };

    validate_cache_root(cache_dir)?;
    let mut result = AddonDownloadCacheRepairResult::for_cache_dir(
        cache_dir.to_path_buf(),
        options.cache_repair_remote_policy,
    );
    if !cache_dir.exists() {
        return Ok(result);
    }

    let files = cache_file_paths(cache_dir)?;
    let mut stats = RemovedPathStats::default();

    for metadata_path in files.iter().filter(|path| is_cache_metadata_path(path)) {
        result.scanned_metadata_count += 1;
        repair_metadata_entry(http_client, metadata_path, options, &mut result, &mut stats)?;
    }

    for file_path in files {
        if is_cache_metadata_path(&file_path) || !file_path.is_file() {
            continue;
        }

        if is_temporary_download_path(&file_path) {
            if remove_path_if_exists(&file_path, &mut stats)? {
                result.partial_download_count += 1;
                result.repaired_entry_count += 1;
            }
            continue;
        }

        if cached_archive_metadata_path(&file_path).is_file() {
            continue;
        }

        if remove_path_if_exists(&file_path, &mut stats)? {
            result.orphan_archive_count += 1;
            result.repaired_entry_count += 1;
        }
    }

    remove_empty_cache_directories(cache_dir, &mut stats)?;
    result.removed_file_count = stats.removed_file_count;
    result.removed_directory_count = stats.removed_directory_count;
    result.reclaimed_bytes = stats.reclaimed_bytes;

    Ok(result)
}

fn repair_metadata_entry(
    http_client: &impl HttpClient,
    metadata_path: &Path,
    options: &AddonProviderOptions,
    result: &mut AddonDownloadCacheRepairResult,
    stats: &mut RemovedPathStats,
) -> AppResult<()> {
    let metadata_bytes = fs::read(metadata_path);
    let metadata = metadata_bytes
        .ok()
        .and_then(|bytes| serde_json::from_slice::<CachedArchiveMetadata>(&bytes).ok());

    let Some(metadata) = metadata else {
        result.invalid_metadata_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        if let Some(archive_path) = archive_path_from_metadata_sidecar(metadata_path) {
            remove_path_if_exists(&archive_path, stats)?;
        }
        return Ok(());
    };

    let Some(archive_path) = archive_path_from_metadata_sidecar(metadata_path) else {
        result.invalid_metadata_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        return Ok(());
    };

    let archive_name = archive_path
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_default();
    let metadata_valid = !metadata.source_display_name.trim().is_empty()
        && !metadata.archive_name.trim().is_empty()
        && metadata.archive_name == archive_name;

    if !metadata_valid {
        result.invalid_metadata_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        remove_path_if_exists(&archive_path, stats)?;
        return Ok(());
    }

    if !archive_path.is_file() {
        result.missing_archive_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        return Ok(());
    }

    let archive_matches = fs::metadata(&archive_path)
        .map(|file_metadata| file_metadata.len() == metadata.file_size)
        .unwrap_or(false)
        && file_sha256(&archive_path)
            .map(|sha256| sha256 == metadata.file_sha256)
            .unwrap_or(false);

    if !archive_matches {
        result.mismatched_archive_count += 1;
        result.repaired_entry_count += 1;
        remove_path_if_exists(metadata_path, stats)?;
        remove_path_if_exists(&archive_path, stats)?;
        return Ok(());
    }

    match repair_remote_cache_entry(http_client, &archive_path, &metadata, options) {
        Ok(CacheRemoteRepairStatus::Unchanged) => {
            result.remote_verified_entry_count += 1;
        }
        Ok(CacheRemoteRepairStatus::Refreshed) => {
            result.remote_refreshed_entry_count += 1;
            result.repaired_entry_count += 1;
        }
        Ok(CacheRemoteRepairStatus::Expired) => {
            result.expired_freshness_entry_count += 1;
            result.repaired_entry_count += 1;
            remove_path_if_exists(metadata_path, stats)?;
            remove_path_if_exists(&archive_path, stats)?;
        }
        Ok(CacheRemoteRepairStatus::Failed) => {
            result.remote_check_failed_count += 1;
            if options.cache_repair_remote_policy.requires_remote_success() {
                return Err(required_remote_repair_error(
                    &metadata,
                    "remote check failed",
                ));
            }
        }
        Err(error) => {
            result.remote_check_failed_count += 1;
            if options.cache_repair_remote_policy.requires_remote_success() {
                return Err(required_remote_repair_error(&metadata, error));
            }
        }
        Ok(CacheRemoteRepairStatus::Skipped) => {
            result.remote_skipped_entry_count += 1;
            if options.cache_repair_remote_policy.requires_remote_success() {
                return Err(required_remote_repair_error(
                    &metadata,
                    "remote validation was skipped",
                ));
            }
        }
    }

    Ok(())
}

fn repair_remote_cache_entry(
    http_client: &impl HttpClient,
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
    options: &AddonProviderOptions,
) -> AppResult<CacheRemoteRepairStatus> {
    if matches!(
        options.cache_repair_remote_policy,
        AddonCacheRepairRemotePolicy::LocalOnly
    ) {
        return Ok(CacheRemoteRepairStatus::Skipped);
    }

    let Some(source_ref) = cached_source_ref_from_metadata(metadata) else {
        return Ok(CacheRemoteRepairStatus::Skipped);
    };

    match source_ref {
        AddonSourceRef::HttpArchive { ref url } => repair_http_archive_cache_entry(
            http_client,
            archive_path,
            metadata,
            &source_ref,
            url,
            options,
        ),
        AddonSourceRef::GitHubRelease {
            ref owner,
            ref repo,
            tag: Some(ref tag),
            asset_name: Some(ref asset_name),
        } => repair_github_archive_cache_entry(
            http_client,
            GitHubCacheEntryRepair {
                archive_path,
                metadata,
                source_ref: &source_ref,
                owner,
                repo,
                tag,
                asset_name,
                options,
            },
        ),
        AddonSourceRef::CurseForgeMod {
            mod_id,
            file_id: Some(file_id),
        } => repair_curseforge_archive_cache_entry(
            http_client,
            archive_path,
            metadata,
            &source_ref,
            mod_id,
            file_id,
            options,
        ),
        _ => Ok(CacheRemoteRepairStatus::Skipped),
    }
}

fn repair_http_archive_cache_entry(
    http_client: &impl HttpClient,
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
    source_ref: &AddonSourceRef,
    url: &str,
    options: &AddonProviderOptions,
) -> AppResult<CacheRemoteRepairStatus> {
    let conditional_headers =
        conditional_request_headers_for_transport_validators(&metadata.remote_validators);
    if !conditional_headers.is_empty() {
        let response = match download_to_path_with_headers(
            http_client,
            url,
            conditional_headers,
            archive_path,
            None,
            None,
        ) {
            Ok(response) => response,
            Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
        };
        if response.is_not_modified() {
            return Ok(CacheRemoteRepairStatus::Unchanged);
        }

        write_cached_archive_metadata(
            archive_path,
            source_ref,
            &metadata.archive_name,
            &remote_validators_for_http_headers(&response.headers),
            options,
        )?;
        return Ok(CacheRemoteRepairStatus::Refreshed);
    }

    if should_reuse_cached_http_archive_without_transport_validators(metadata, options) {
        return Ok(CacheRemoteRepairStatus::Skipped);
    }

    Ok(CacheRemoteRepairStatus::Expired)
}

fn repair_github_archive_cache_entry(
    http_client: &impl HttpClient,
    request: GitHubCacheEntryRepair<'_>,
) -> AppResult<CacheRemoteRepairStatus> {
    let release = match fetch_github_release_with_client(
        http_client,
        request.owner,
        request.repo,
        Some(request.tag),
    ) {
        Ok(release) => release,
        Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
    };
    let asset = match select_github_release_asset(&release, Some(request.asset_name)) {
        Ok(asset) => asset,
        Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
    };
    let remote_validators = remote_validators_for_github_asset(asset);
    if remote_validators.is_empty() {
        return Ok(CacheRemoteRepairStatus::Skipped);
    }
    if remote_validators == request.metadata.remote_validators {
        return Ok(CacheRemoteRepairStatus::Unchanged);
    }

    refresh_cached_archive(
        http_client,
        CachedArchiveRefresh {
            archive_path: request.archive_path,
            source_ref: request.source_ref,
            archive_name: &request.metadata.archive_name,
            download_url: &asset.browser_download_url,
            headers: Vec::new(),
            remote_validators: &remote_validators,
            options: request.options,
        },
    )?;
    Ok(CacheRemoteRepairStatus::Refreshed)
}

fn repair_curseforge_archive_cache_entry(
    http_client: &impl HttpClient,
    archive_path: &Path,
    metadata: &CachedArchiveMetadata,
    source_ref: &AddonSourceRef,
    mod_id: u32,
    file_id: u32,
    options: &AddonProviderOptions,
) -> AppResult<CacheRemoteRepairStatus> {
    let file =
        match resolve_curseforge_file_with_client(http_client, mod_id, Some(file_id), None, None) {
            Ok(file) => file,
            Err(_) => return Ok(CacheRemoteRepairStatus::Failed),
        };
    let Some(download_url) = file.download_url.clone() else {
        return Ok(CacheRemoteRepairStatus::Failed);
    };
    let remote_validators = remote_validators_for_curseforge_file(&file);
    if remote_validators.is_empty() {
        return Ok(CacheRemoteRepairStatus::Skipped);
    }
    if remote_validators == metadata.remote_validators {
        return Ok(CacheRemoteRepairStatus::Unchanged);
    }

    refresh_cached_archive(
        http_client,
        CachedArchiveRefresh {
            archive_path,
            source_ref,
            archive_name: &metadata.archive_name,
            download_url: &download_url,
            headers: Vec::new(),
            remote_validators: &remote_validators,
            options,
        },
    )?;
    Ok(CacheRemoteRepairStatus::Refreshed)
}

fn refresh_cached_archive(
    http_client: &impl HttpClient,
    request: CachedArchiveRefresh<'_>,
) -> AppResult<()> {
    download_to_path_with_headers(
        http_client,
        request.download_url,
        request.headers,
        request.archive_path,
        None,
        None,
    )?;
    write_cached_archive_metadata(
        request.archive_path,
        request.source_ref,
        request.archive_name,
        request.remote_validators,
        request.options,
    )
}

fn required_remote_repair_error(
    metadata: &CachedArchiveMetadata,
    detail: impl std::fmt::Display,
) -> AppError {
    AppError::Validation(format!(
        "addon cache repair remote validation is required for `{}` but failed: {detail}",
        metadata.source_display_name
    ))
}

fn cached_source_ref_from_metadata(metadata: &CachedArchiveMetadata) -> Option<AddonSourceRef> {
    metadata
        .source_ref
        .clone()
        .or_else(|| parse_cached_source_display_name(&metadata.source_display_name))
}

fn parse_cached_source_display_name(source_display_name: &str) -> Option<AddonSourceRef> {
    if let Ok(Some(source_ref)) = parse_curseforge_source(source_display_name) {
        return Some(source_ref);
    }
    if let Ok(Some(source_ref)) = parse_github_source(source_display_name) {
        return Some(source_ref);
    }
    if let Ok(Some(source_ref)) = parse_tukui_source(source_display_name) {
        return Some(source_ref);
    }
    if is_http_url(source_display_name) {
        return Some(AddonSourceRef::HttpArchive {
            url: source_display_name.to_string(),
        });
    }

    None
}
