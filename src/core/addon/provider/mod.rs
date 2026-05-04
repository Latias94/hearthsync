mod cache;
mod curseforge;
mod default_provider;
mod github;
mod http;
mod materialize;
mod parse;
mod registry;
mod source;
mod source_adapter;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
mod validation;

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Serialize;

pub use self::cache::{
    AddonDownloadCachePurgeResult, AddonDownloadCacheRepairResult, HttpNoValidatorCachePolicy,
};
pub use self::default_provider::{
    AddonProviderOptions, AddonProviderRetryPolicy, DefaultAddonProvider,
};
pub use self::registry::{
    AddonProviderDescriptor, AddonProviderOperationCapabilities, AddonProviderPolicyCapabilities,
    AddonProviderSourceCapability, AddonSourceFamily,
};
pub use self::source::AddonSourceRef;
pub(crate) use self::source::{
    addon_source_input_is_local_archive, canonicalize_local_archive_path,
    validate_absolute_local_archive_source_path, validate_addon_source_ref,
};
use super::policy::AddonReleaseChannel;
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;
use crate::core::task::CancellationToken;

#[derive(Debug)]
pub struct MaterializedAddonSource {
    pub source_ref: AddonSourceRef,
    pub archive_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub struct MaterializeSourceInputRequest<'a> {
    pub source: &'a str,
    pub stage_root: &'a Path,
    pub context: AddonProviderContext<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct MaterializeSourceRefRequest<'a> {
    pub source: &'a AddonSourceRef,
    pub stage_root: &'a Path,
    pub context: AddonProviderContext<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct ResolveAddonDependenciesRequest<'a> {
    pub source: &'a AddonSourceRef,
    pub context: AddonProviderContext<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonDependencyResolutionStrategy {
    MissingRequiredOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonDependencyResolutionCapability {
    Unsupported,
    Supported {
        strategy: AddonDependencyResolutionStrategy,
    },
}

impl AddonDependencyResolutionCapability {
    pub fn missing_required_only() -> Self {
        Self::Supported {
            strategy: AddonDependencyResolutionStrategy::MissingRequiredOnly,
        }
    }

    pub fn supported_strategy(self) -> Option<AddonDependencyResolutionStrategy> {
        match self {
            Self::Unsupported => None,
            Self::Supported { strategy } => Some(strategy),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAddonDependencies {
    pub strategy: AddonDependencyResolutionStrategy,
    pub dependencies: Vec<AddonSourceRef>,
}

impl ResolvedAddonDependencies {
    pub fn missing_required_only(dependencies: Vec<AddonSourceRef>) -> Self {
        Self {
            strategy: AddonDependencyResolutionStrategy::MissingRequiredOnly,
            dependencies,
        }
    }
}

pub trait AddonDownloadProgressObserver {
    fn on_download_progress(
        &self,
        source: &AddonSourceRef,
        archive_name: &str,
        bytes_current: u64,
        bytes_total: Option<u64>,
        bytes_per_second: Option<u64>,
    );
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AddonSourceResolutionPolicy {
    pub release_channel: Option<AddonReleaseChannel>,
    pub allow_prerelease: Option<bool>,
}

#[derive(Clone, Copy, Default)]
pub struct AddonProviderContext<'a> {
    pub target_flavor: Option<WowFlavor>,
    pub cancellation: Option<&'a dyn CancellationToken>,
    download_progress: Option<&'a dyn AddonDownloadProgressObserver>,
    resolution_policy: AddonSourceResolutionPolicy,
}

impl fmt::Debug for AddonProviderContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AddonProviderContext")
            .field("target_flavor", &self.target_flavor)
            .field("has_cancellation", &self.cancellation.is_some())
            .field("has_download_progress", &self.download_progress.is_some())
            .field("resolution_policy", &self.resolution_policy)
            .finish()
    }
}

impl<'a> AddonProviderContext<'a> {
    pub fn new(
        target_flavor: Option<WowFlavor>,
        cancellation: Option<&'a dyn CancellationToken>,
    ) -> Self {
        Self {
            target_flavor,
            cancellation,
            download_progress: None,
            resolution_policy: AddonSourceResolutionPolicy::default(),
        }
    }

    pub(crate) fn with_download_progress(
        mut self,
        download_progress: Option<&'a dyn AddonDownloadProgressObserver>,
    ) -> Self {
        self.download_progress = download_progress;
        self
    }

    pub(crate) fn with_resolution_policy(
        mut self,
        resolution_policy: AddonSourceResolutionPolicy,
    ) -> Self {
        self.resolution_policy = resolution_policy;
        self
    }

    pub fn resolution_policy(&self) -> AddonSourceResolutionPolicy {
        self.resolution_policy
    }

    pub fn report_download_progress(
        &self,
        source: &AddonSourceRef,
        archive_name: &str,
        bytes_current: u64,
        bytes_total: Option<u64>,
        bytes_per_second: Option<u64>,
    ) {
        if let Some(observer) = self.download_progress {
            observer.on_download_progress(
                source,
                archive_name,
                bytes_current,
                bytes_total,
                bytes_per_second,
            );
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddonSearchRequest<'a> {
    pub query: &'a str,
    pub flavor: WowFlavor,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchResult {
    pub provider: &'static str,
    pub name: String,
    pub summary: Option<String>,
    pub source: AddonSourceRef,
    pub install_hint: String,
    pub website_url: Option<String>,
    pub provider_project_id: Option<u32>,
    pub provider_file_id: Option<u32>,
    pub download_count: u64,
}

pub trait AddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource>;

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource>;

    fn dependency_resolution_capability(
        &self,
        _source: &AddonSourceRef,
    ) -> AddonDependencyResolutionCapability {
        AddonDependencyResolutionCapability::Unsupported
    }

    fn provider_descriptors(&self) -> Vec<AddonProviderDescriptor> {
        Vec::new()
    }

    fn source_capabilities(&self) -> Vec<AddonProviderSourceCapability> {
        self.provider_descriptors()
            .into_iter()
            .map(AddonProviderDescriptor::source_capability)
            .collect()
    }

    fn resolve_addon_dependencies(
        &self,
        _request: ResolveAddonDependenciesRequest<'_>,
    ) -> AppResult<ResolvedAddonDependencies> {
        Err(AppError::Validation(
            "addon dependency installation is not supported by this provider".to_string(),
        ))
    }

    fn purge_download_cache(&self) -> AppResult<AddonDownloadCachePurgeResult> {
        Err(AppError::Validation(
            "addon provider does not support download cache management".to_string(),
        ))
    }

    fn repair_download_cache(&self) -> AppResult<AddonDownloadCacheRepairResult> {
        Err(AppError::Validation(
            "addon provider does not support download cache management".to_string(),
        ))
    }

    fn search_addons(&self, request: AddonSearchRequest<'_>) -> AppResult<Vec<AddonSearchResult>>;
}
