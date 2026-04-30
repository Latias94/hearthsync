mod download;
mod maintenance;
mod metadata;
mod policy;
mod repair;

pub use self::maintenance::AddonDownloadCachePurgeResult;
pub use self::policy::HttpNoValidatorCachePolicy;
pub use self::repair::AddonDownloadCacheRepairResult;

pub(super) use self::download::{
    download_to_path_with_headers, guess_archive_name_from_url, resolve_archive_path,
};
pub(super) use self::maintenance::purge_download_cache_dir;
#[cfg(test)]
pub(super) use self::metadata::{CachedArchiveMetadata, cached_archive_metadata_path};
pub(super) use self::metadata::{
    cached_archive_metadata_if_local_file_matches, write_cached_archive_metadata,
};
pub(super) use self::policy::{
    should_reuse_cached_archive, should_reuse_cached_http_archive_without_transport_validators,
};
pub(super) use self::repair::repair_download_cache_dir;

#[cfg(test)]
mod tests;
