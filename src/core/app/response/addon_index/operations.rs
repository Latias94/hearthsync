use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::index::{
    AddonIndexInstallResult as DomainAddonIndexInstallResult,
    AddonIndexRelinkResult as DomainAddonIndexRelinkResult,
    AddonIndexUpdateResult as DomainAddonIndexUpdateResult,
};
use crate::core::app::AddonPackageMetadataValue;

use super::super::super::map_owned_vec;
use super::super::addon::{
    AddonSourceResult, InstalledAddonPackageResult, TrackedAddonResult, UpdatedAddonPackageResult,
};
use super::package::AddonIndexPackageResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInstallResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackageResult,
    pub install: InstalledAddonPackageResult,
}

impl AddonIndexInstallResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonIndexInstallResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            index_path: value.index_path,
            package: AddonIndexPackageResult::from_domain_with_provider(value.package, provider),
            install: InstalledAddonPackageResult::from_domain_with_provider(
                value.install,
                provider,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexRelinkResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackageResult,
    pub dry_run: bool,
    pub tracked_package_id: String,
    pub previous_source: AddonSourceResult,
    pub previous_source_label: String,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub metadata: AddonPackageMetadataValue,
    pub registry_path: PathBuf,
    pub source_changed: bool,
    pub metadata_changed: bool,
}

impl AddonIndexRelinkResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonIndexRelinkResult,
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
            index_path: value.index_path,
            package: AddonIndexPackageResult::from_domain_with_provider(value.package, provider),
            dry_run: value.dry_run,
            tracked_package_id: value.tracked_package_id,
            previous_source,
            previous_source_label,
            source,
            source_label,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
            metadata: AddonPackageMetadataValue::from_domain(value.metadata),
            registry_path: value.registry_path,
            source_changed: value.source_changed,
            metadata_changed: value.metadata_changed,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexUpdateResult {
    pub index_path: PathBuf,
    pub selected_package_count: usize,
    pub selected_packages: Vec<AddonIndexPackageResult>,
    pub update: UpdatedAddonPackageResult,
}

impl AddonIndexUpdateResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonIndexUpdateResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let selected_package_count = value.selected_packages.len();

        Self {
            index_path: value.index_path,
            selected_package_count,
            selected_packages: map_owned_vec(value.selected_packages, |value| {
                AddonIndexPackageResult::from_domain_with_provider(value, provider)
            }),
            update: UpdatedAddonPackageResult::from_domain_with_provider(value.update, provider),
        }
    }
}
