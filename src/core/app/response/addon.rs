use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::{
    AddonInventory, AddonSearchCatalog as DomainAddonSearchCatalog,
    AddonSearchResult as DomainAddonSearchResult, AddonSourceRef as DomainAddonSourceRef,
    InstalledAddonPackageResult as DomainInstalledAddonPackageResult,
    RemovedAddonPackageResult as DomainRemovedAddonPackageResult, TrackedAddon,
    TrackedAddonPackage, UpdatedAddonPackageResult as DomainUpdatedAddonPackageResult,
};
use crate::core::app::AddonPackageMetadataValue;

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
    pub(crate) fn from_domain(value: DomainAddonSourceRef) -> Self {
        let display_name = value.display_name();

        match value {
            DomainAddonSourceRef::LocalArchive { path } => Self {
                kind: AddonSourceKindResult::LocalArchive,
                display_name,
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
    pub(crate) fn from_domain(value: TrackedAddonPackage) -> Self {
        let source = AddonSourceResult::from_domain(value.source);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            source,
            source_label,
            installed_at: value.installed_at,
            updated_at: value.updated_at,
            addon_count,
            addons: value
                .addons
                .into_iter()
                .map(TrackedAddonResult::from_domain)
                .collect(),
            metadata: value.metadata.map(AddonPackageMetadataValue::from),
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
    pub(crate) fn from_domain(value: AddonInventory) -> Self {
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
            tracked_packages: value
                .tracked_packages
                .into_iter()
                .map(TrackedAddonPackageResult::from_domain)
                .collect(),
            untracked_addons: value.untracked_addons,
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
    pub(crate) fn from_domain(value: DomainAddonSearchResult) -> Self {
        let source = AddonSourceResult::from_domain(value.source);
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
    pub(crate) fn from_domain(value: DomainAddonSearchCatalog) -> Self {
        let result_count = value.results.len();

        Self {
            query: value.query,
            result_count,
            results: value
                .results
                .into_iter()
                .map(AddonSearchResult::from_domain)
                .collect(),
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
    pub(crate) fn from_domain(value: DomainInstalledAddonPackageResult) -> Self {
        let source = AddonSourceResult::from_domain(value.source);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();
        let replaced_addon_count = value.replaced_addons.len();

        Self {
            dry_run: value.dry_run,
            source,
            source_label,
            package_id: value.package_id,
            addon_count,
            addons: value
                .addons
                .into_iter()
                .map(TrackedAddonResult::from_domain)
                .collect(),
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
    pub backup_path: Option<PathBuf>,
}

impl UpdatedAddonPackageResult {
    pub(crate) fn from_domain(value: DomainUpdatedAddonPackageResult) -> Self {
        let updated_package_count = value.updated_packages.len();

        Self {
            dry_run: value.dry_run,
            registry_path: value.registry_path,
            files_to_write: value.files_to_write,
            written_files: value.written_files,
            updated_package_count,
            updated_packages: value
                .updated_packages
                .into_iter()
                .map(TrackedAddonPackageResult::from_domain)
                .collect(),
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
    pub(crate) fn from_domain(value: DomainRemovedAddonPackageResult) -> Self {
        let removed_package_count = value.removed_packages.len();
        let removed_addon_count = value.removed_addons.len();

        Self {
            dry_run: value.dry_run,
            registry_path: value.registry_path,
            removed_package_count,
            removed_packages: value
                .removed_packages
                .into_iter()
                .map(TrackedAddonPackageResult::from_domain)
                .collect(),
            removed_addon_count,
            removed_addons: value.removed_addons,
            registry_cleaned: value.registry_cleaned,
            backup_path: value.backup_path,
        }
    }
}
