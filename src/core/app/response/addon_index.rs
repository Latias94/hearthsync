use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::index::{
    AddonIndexInspection, AddonIndexInstallResult as DomainAddonIndexInstallResult,
    AddonIndexPackage, AddonIndexUpdateResult as DomainAddonIndexUpdateResult,
};

use super::addon::{AddonSourceResult, InstalledAddonPackageResult, UpdatedAddonPackageResult};
use super::map_domain_vec;

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexPackageResult {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub sha256: Option<String>,
    pub addon_directories: Vec<String>,
    pub supported_flavors: Vec<String>,
}

impl AddonIndexPackageResult {
    pub(crate) fn from_domain(value: AddonIndexPackage) -> Self {
        let source = AddonSourceResult::from_domain(value.source);
        let source_label = source.display_name.clone();

        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            sha256: value.sha256,
            addon_directories: value.addon_directories,
            supported_flavors: value.supported_flavors,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspectionResult {
    pub index_path: PathBuf,
    pub name: String,
    pub description: Option<String>,
    pub package_count: usize,
    pub packages: Vec<AddonIndexPackageResult>,
}

impl AddonIndexInspectionResult {
    pub(crate) fn from_domain(value: AddonIndexInspection) -> Self {
        Self {
            index_path: value.index_path,
            name: value.index.name,
            description: value.index.description,
            package_count: value.package_count,
            packages: map_domain_vec(value.index.packages, AddonIndexPackageResult::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInstallResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackageResult,
    pub install: InstalledAddonPackageResult,
}

impl AddonIndexInstallResult {
    pub(crate) fn from_domain(value: DomainAddonIndexInstallResult) -> Self {
        Self {
            index_path: value.index_path,
            package: AddonIndexPackageResult::from_domain(value.package),
            install: InstalledAddonPackageResult::from_domain(value.install),
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
    pub(crate) fn from_domain(value: DomainAddonIndexUpdateResult) -> Self {
        let selected_package_count = value.selected_packages.len();

        Self {
            index_path: value.index_path,
            selected_package_count,
            selected_packages: map_domain_vec(
                value.selected_packages,
                AddonIndexPackageResult::from_domain,
            ),
            update: UpdatedAddonPackageResult::from_domain(value.update),
        }
    }
}
