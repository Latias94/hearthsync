use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::{
    AddonProvider, AdoptedAddonPackageResult as DomainAdoptedAddonPackageResult,
    InstalledAddonPackageResult as DomainInstalledAddonPackageResult,
    RelinkedAddonPackageResult as DomainRelinkedAddonPackageResult,
    RemovedAddonPackageResult as DomainRemovedAddonPackageResult,
    UpdatedAddonPackageResult as DomainUpdatedAddonPackageResult,
};

use super::super::super::map_owned_vec;
use super::source::AddonSourceResult;
use super::tracked::{TrackedAddonPackageResult, TrackedAddonResult};

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
