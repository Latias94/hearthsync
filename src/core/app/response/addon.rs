use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::{
    AddonDownloadCachePurgeResult as DomainAddonDownloadCachePurgeResult,
    AddonDownloadCacheRepairResult as DomainAddonDownloadCacheRepairResult, AddonInventory,
    AddonProvider, AddonSearchCatalog as DomainAddonSearchCatalog,
    AddonSearchResult as DomainAddonSearchResult, AddonSourceRef as DomainAddonSourceRef,
    AdoptedAddonPackageResult as DomainAdoptedAddonPackageResult,
    InstalledAddonPackageResult as DomainInstalledAddonPackageResult,
    RelinkedAddonPackageResult as DomainRelinkedAddonPackageResult,
    RemovedAddonPackageResult as DomainRemovedAddonPackageResult, TrackedAddon,
    TrackedAddonPackage, UpdatedAddonPackageResult as DomainUpdatedAddonPackageResult,
};
use crate::core::app::{AddonDependencyResolutionCapabilityValue, AddonPackageMetadataValue};

use super::super::map_owned_vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonSourceKindResult {
    LocalArchive,
    HttpArchive,
    CurseForgeMod,
    GitHubRelease,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSourceResult {
    pub kind: AddonSourceKindResult,
    pub display_name: String,
    pub dependency_resolution_capability: AddonDependencyResolutionCapabilityValue,
    pub local_archive_path: Option<PathBuf>,
    pub url: Option<String>,
    pub mod_id: Option<u32>,
    pub file_id: Option<u32>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub tag: Option<String>,
    pub asset_name: Option<String>,
}

impl AddonSourceResult {
    pub(crate) fn from_domain_with_provider<P>(value: DomainAddonSourceRef, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let display_name = value.display_name();
        let dependency_resolution_capability =
            AddonDependencyResolutionCapabilityValue::from_domain(
                provider.dependency_resolution_capability(&value),
            );

        match value {
            DomainAddonSourceRef::LocalArchive { path } => Self {
                kind: AddonSourceKindResult::LocalArchive,
                display_name,
                dependency_resolution_capability,
                local_archive_path: Some(path),
                url: None,
                mod_id: None,
                file_id: None,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
            },
            DomainAddonSourceRef::HttpArchive { url } => Self {
                kind: AddonSourceKindResult::HttpArchive,
                display_name,
                dependency_resolution_capability,
                local_archive_path: None,
                url: Some(url),
                mod_id: None,
                file_id: None,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
            },
            DomainAddonSourceRef::CurseForgeMod { mod_id, file_id } => Self {
                kind: AddonSourceKindResult::CurseForgeMod,
                display_name,
                dependency_resolution_capability,
                local_archive_path: None,
                url: None,
                mod_id: Some(mod_id),
                file_id,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
            },
            DomainAddonSourceRef::GitHubRelease {
                owner,
                repo,
                tag,
                asset_name,
            } => Self {
                kind: AddonSourceKindResult::GitHubRelease,
                display_name,
                dependency_resolution_capability,
                local_archive_path: None,
                url: None,
                mod_id: None,
                file_id: None,
                owner: Some(owner),
                repo: Some(repo),
                tag,
                asset_name,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackedAddonResult {
    pub directory_name: String,
    pub toc_file: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
}

impl TrackedAddonResult {
    pub(crate) fn from_domain(value: TrackedAddon) -> Self {
        Self {
            directory_name: value.directory_name,
            toc_file: value.toc_file,
            title: value.title,
            version: value.version,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackedAddonPackageResult {
    pub package_id: String,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub metadata: Option<AddonPackageMetadataValue>,
}

impl TrackedAddonPackageResult {
    pub(crate) fn from_domain_with_provider<P>(value: TrackedAddonPackage, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            source,
            source_label,
            installed_at: value.installed_at,
            updated_at: value.updated_at,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
            metadata: value.metadata.map(AddonPackageMetadataValue::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonInventoryResult {
    pub target_addon_root: PathBuf,
    pub registry_path: PathBuf,
    pub tracked_package_count: usize,
    pub tracked_addon_count: usize,
    pub tracked_packages: Vec<TrackedAddonPackageResult>,
    pub untracked_addons: Vec<String>,
}

impl AddonInventoryResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonInventory, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let tracked_package_count = value.tracked_packages.len();
        let tracked_addon_count = value
            .tracked_packages
            .iter()
            .map(|package| package.addons.len())
            .sum();

        Self {
            target_addon_root: value.target_addon_root,
            registry_path: value.registry_path,
            tracked_package_count,
            tracked_addon_count,
            tracked_packages: map_owned_vec(value.tracked_packages, |value| {
                TrackedAddonPackageResult::from_domain_with_provider(value, provider)
            }),
            untracked_addons: value.untracked_addons,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AdoptedAddonPackageResult {
    pub dry_run: bool,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub package_id: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub registry_path: PathBuf,
}

impl AdoptedAddonPackageResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAdoptedAddonPackageResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            dry_run: value.dry_run,
            source,
            source_label,
            package_id: value.package_id,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
            registry_path: value.registry_path,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RelinkedAddonPackageResult {
    pub dry_run: bool,
    pub package_id: String,
    pub previous_source: AddonSourceResult,
    pub previous_source_label: String,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub registry_path: PathBuf,
    pub cleared_metadata: bool,
}

impl RelinkedAddonPackageResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainRelinkedAddonPackageResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let previous_source =
            AddonSourceResult::from_domain_with_provider(value.previous_source, provider);
        let previous_source_label = previous_source.display_name.clone();
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            dry_run: value.dry_run,
            package_id: value.package_id,
            previous_source,
            previous_source_label,
            source,
            source_label,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
            registry_path: value.registry_path,
            cleared_metadata: value.cleared_metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchResult {
    pub provider: String,
    pub name: String,
    pub summary: Option<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub install_hint: String,
    pub website_url: Option<String>,
    pub provider_project_id: Option<u32>,
    pub provider_file_id: Option<u32>,
    pub download_count: u64,
}

impl AddonSearchResult {
    pub(crate) fn from_domain_with_provider<P>(value: DomainAddonSearchResult, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();

        Self {
            provider: value.provider.to_string(),
            name: value.name,
            summary: value.summary,
            source,
            source_label,
            install_hint: value.install_hint,
            website_url: value.website_url,
            provider_project_id: value.provider_project_id,
            provider_file_id: value.provider_file_id,
            download_count: value.download_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchCatalogResult {
    pub query: String,
    pub result_count: usize,
    pub results: Vec<AddonSearchResult>,
}

impl AddonSearchCatalogResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonSearchCatalog,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let result_count = value.results.len();

        Self {
            query: value.query,
            result_count,
            results: map_owned_vec(value.results, |value| {
                AddonSearchResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonCachePurgeResult {
    pub configured: bool,
    pub cache_dir: Option<PathBuf>,
    pub removed_file_count: usize,
    pub removed_directory_count: usize,
    pub reclaimed_bytes: u64,
}

impl AddonCachePurgeResult {
    pub(crate) fn from_domain(value: DomainAddonDownloadCachePurgeResult) -> Self {
        Self {
            configured: value.cache_dir.is_some(),
            cache_dir: value.cache_dir,
            removed_file_count: value.removed_file_count,
            removed_directory_count: value.removed_directory_count,
            reclaimed_bytes: value.reclaimed_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonCacheRepairResult {
    pub configured: bool,
    pub cache_dir: Option<PathBuf>,
    pub scanned_metadata_count: usize,
    pub repaired_entry_count: usize,
    pub invalid_metadata_count: usize,
    pub missing_archive_count: usize,
    pub mismatched_archive_count: usize,
    pub orphan_archive_count: usize,
    pub partial_download_count: usize,
    pub remote_verified_entry_count: usize,
    pub remote_refreshed_entry_count: usize,
    pub remote_check_failed_count: usize,
    pub expired_freshness_entry_count: usize,
    pub removed_file_count: usize,
    pub removed_directory_count: usize,
    pub reclaimed_bytes: u64,
}

impl AddonCacheRepairResult {
    pub(crate) fn from_domain(value: DomainAddonDownloadCacheRepairResult) -> Self {
        Self {
            configured: value.cache_dir.is_some(),
            cache_dir: value.cache_dir,
            scanned_metadata_count: value.scanned_metadata_count,
            repaired_entry_count: value.repaired_entry_count,
            invalid_metadata_count: value.invalid_metadata_count,
            missing_archive_count: value.missing_archive_count,
            mismatched_archive_count: value.mismatched_archive_count,
            orphan_archive_count: value.orphan_archive_count,
            partial_download_count: value.partial_download_count,
            remote_verified_entry_count: value.remote_verified_entry_count,
            remote_refreshed_entry_count: value.remote_refreshed_entry_count,
            remote_check_failed_count: value.remote_check_failed_count,
            expired_freshness_entry_count: value.expired_freshness_entry_count,
            removed_file_count: value.removed_file_count,
            removed_directory_count: value.removed_directory_count,
            reclaimed_bytes: value.reclaimed_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledAddonPackageResult {
    pub dry_run: bool,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub package_id: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub files_to_write: usize,
    pub written_files: usize,
    pub replaced_addon_count: usize,
    pub replaced_addons: Vec<String>,
    pub registry_path: PathBuf,
    pub backup_path: Option<PathBuf>,
}

impl InstalledAddonPackageResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainInstalledAddonPackageResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();
        let replaced_addon_count = value.replaced_addons.len();

        Self {
            dry_run: value.dry_run,
            source,
            source_label,
            package_id: value.package_id,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
            files_to_write: value.files_to_write,
            written_files: value.written_files,
            replaced_addon_count,
            replaced_addons: value.replaced_addons,
            registry_path: value.registry_path,
            backup_path: value.backup_path,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatedAddonPackageResult {
    pub dry_run: bool,
    pub registry_path: PathBuf,
    pub files_to_write: usize,
    pub written_files: usize,
    pub updated_package_count: usize,
    pub updated_packages: Vec<TrackedAddonPackageResult>,
    pub installed_dependency_package_count: usize,
    pub installed_dependency_packages: Vec<TrackedAddonPackageResult>,
    pub ignored_package_count: usize,
    pub ignored_packages: Vec<String>,
    pub backup_path: Option<PathBuf>,
}

impl UpdatedAddonPackageResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainUpdatedAddonPackageResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let updated_package_count = value.updated_packages.len();
        let installed_dependency_package_count = value.installed_dependency_packages.len();
        let ignored_package_count = value.ignored_packages.len();

        Self {
            dry_run: value.dry_run,
            registry_path: value.registry_path,
            files_to_write: value.files_to_write,
            written_files: value.written_files,
            updated_package_count,
            updated_packages: map_owned_vec(value.updated_packages, |value| {
                TrackedAddonPackageResult::from_domain_with_provider(value, provider)
            }),
            installed_dependency_package_count,
            installed_dependency_packages: map_owned_vec(
                value.installed_dependency_packages,
                |value| TrackedAddonPackageResult::from_domain_with_provider(value, provider),
            ),
            ignored_package_count,
            ignored_packages: value.ignored_packages,
            backup_path: value.backup_path,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RemovedAddonPackageResult {
    pub dry_run: bool,
    pub registry_path: PathBuf,
    pub removed_package_count: usize,
    pub removed_packages: Vec<TrackedAddonPackageResult>,
    pub removed_addon_count: usize,
    pub removed_addons: Vec<String>,
    pub registry_cleaned: bool,
    pub backup_path: Option<PathBuf>,
}

impl RemovedAddonPackageResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainRemovedAddonPackageResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let removed_package_count = value.removed_packages.len();
        let removed_addon_count = value.removed_addons.len();

        Self {
            dry_run: value.dry_run,
            registry_path: value.registry_path,
            removed_package_count,
            removed_packages: map_owned_vec(value.removed_packages, |value| {
                TrackedAddonPackageResult::from_domain_with_provider(value, provider)
            }),
            removed_addon_count,
            removed_addons: value.removed_addons,
            registry_cleaned: value.registry_cleaned,
            backup_path: value.backup_path,
        }
    }
}
