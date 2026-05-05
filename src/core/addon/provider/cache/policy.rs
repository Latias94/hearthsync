use std::path::Path;

use super::super::validation::RemoteArchiveValidators;
use super::super::{AddonProviderOptions, AddonSourceRef};
use super::metadata::{
    CachedArchiveMetadata, cached_archive_matches_metadata, current_unix_timestamp_secs,
};

const DEFAULT_HTTP_NO_VALIDATOR_CACHE_WINDOW_SECS: u64 = 15 * 60;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AddonCacheRepairRemotePolicy {
    LocalOnly,
    #[default]
    ValidateRemote,
    RequireRemote,
}

impl AddonCacheRepairRemotePolicy {
    pub(crate) fn requires_remote_success(self) -> bool {
        matches!(self, Self::RequireRemote)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpNoValidatorCachePolicy {
    AlwaysRefresh,
    ReuseWithinWindow { max_age_secs: u64 },
}

impl Default for HttpNoValidatorCachePolicy {
    fn default() -> Self {
        Self::ReuseWithinWindow {
            max_age_secs: DEFAULT_HTTP_NO_VALIDATOR_CACHE_WINDOW_SECS,
        }
    }
}

impl HttpNoValidatorCachePolicy {
    fn max_age_secs(&self) -> Option<u64> {
        match self {
            Self::AlwaysRefresh => None,
            Self::ReuseWithinWindow { max_age_secs } => Some(*max_age_secs),
        }
    }
}

pub(in crate::core::addon::provider) fn should_reuse_cached_archive(
    source: &AddonSourceRef,
    archive_name: &str,
    remote_validators: &RemoteArchiveValidators,
    archive_path: &Path,
    options: &AddonProviderOptions,
) -> bool {
    options.download_cache_dir.is_some()
        && archive_path.is_file()
        && cached_archive_matches_metadata(archive_path, source, archive_name, remote_validators)
}

pub(in crate::core::addon::provider) fn should_reuse_cached_http_archive_without_transport_validators(
    metadata: &CachedArchiveMetadata,
    options: &AddonProviderOptions,
) -> bool {
    let Some(max_age_secs) = options.http_no_validator_cache_policy.max_age_secs() else {
        return false;
    };
    let Some(fetched_at_unix_timestamp) = metadata.fetched_at_unix_timestamp else {
        return false;
    };
    let Ok(now) = current_unix_timestamp_secs() else {
        return false;
    };
    if fetched_at_unix_timestamp > now {
        return false;
    }

    now - fetched_at_unix_timestamp <= max_age_secs
}
