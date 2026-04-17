mod matching;
mod operations;
mod storage;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::addon::{AddonSourceRef, InstalledAddonPackageResult, UpdatedAddonPackageResult};
use crate::core::install::DetectedFlavorInstallation;

pub use self::operations::{
    install_addon_from_index, install_addon_from_index_task, update_addons_from_index,
    update_addons_from_index_task,
};
pub(crate) use self::operations::{
    install_addon_from_index_task_with_provider, update_addons_from_index_task_with_provider,
};
pub use self::storage::inspect_addon_index;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonIndex {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub packages: Vec<AddonIndexPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonIndexPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: AddonSourceRef,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub addon_directories: Vec<String>,
    #[serde(default)]
    pub supported_flavors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspection {
    pub index_path: PathBuf,
    pub index: AddonIndex,
    pub package_count: usize,
}

#[derive(Debug, Clone)]
pub struct AddonIndexInstallRequest {
    pub installation: DetectedFlavorInstallation,
    pub index_path: PathBuf,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInstallResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackage,
    pub install: InstalledAddonPackageResult,
}

#[derive(Debug, Clone)]
pub struct AddonIndexUpdateRequest {
    pub installation: DetectedFlavorInstallation,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexUpdateResult {
    pub index_path: PathBuf,
    pub selected_packages: Vec<AddonIndexPackage>,
    pub update: UpdatedAddonPackageResult,
}
