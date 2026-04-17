mod execution;
pub mod index;
pub mod lock;
mod mutation;
mod package_prep;
mod provider;
mod registry;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

pub use self::execution::{
    install_addon, install_addon_task, remove_addons, remove_addons_task, update_addons,
    update_addons_task,
};
use self::package_prep::find_primary_toc;
use self::provider::AddonSearchRequest as ProviderAddonSearchRequest;
pub use self::provider::{
    AddonProvider, AddonProviderContext, AddonProviderOptions, AddonProviderRetryPolicy,
    AddonSearchRequest, AddonSearchResult, AddonSourceRef, DefaultAddonProvider,
    MaterializeSourceInputRequest, MaterializeSourceRefRequest, MaterializedAddonSource,
};
use self::registry::registry_path;
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

pub(crate) use self::execution::{
    InstallAddonExecutionPlan, InstallPreparedAddonRequest, execute_install_plan_task,
    install_addon_task_with_provider, prepare_install_prepared_addon,
    update_addons_task_with_provider,
};
pub(crate) use self::mutation::{
    install_prepared_package, remove_selected_packages, rollback_or_report_addon_error,
    update_prepared_packages,
};
pub(crate) use self::package_prep::{
    prepare_package_from_archive_with_source, prepare_package_from_source_input_with_provider,
    prepare_package_from_source_ref_with_provider,
};
pub(crate) use self::provider::canonicalize_local_archive_path;
pub(crate) use self::registry::{load_registry, save_registry};

#[derive(Debug, Clone, Serialize)]
pub struct AddonInventory {
    pub target_addon_root: PathBuf,
    pub registry_path: PathBuf,
    pub tracked_packages: Vec<TrackedAddonPackage>,
    pub untracked_addons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub source: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub metadata: Option<AddonPackageMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledAddonPackageResult {
    pub dry_run: bool,
    pub source: AddonSourceRef,
    pub package_id: String,
    pub addons: Vec<TrackedAddon>,
    pub files_to_write: usize,
    pub written_files: usize,
    pub replaced_addons: Vec<String>,
    pub registry_path: PathBuf,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoveAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub query: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchCatalog {
    pub query: String,
    pub results: Vec<AddonSearchResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatedAddonPackageResult {
    pub dry_run: bool,
    pub registry_path: PathBuf,
    pub files_to_write: usize,
    pub written_files: usize,
    pub updated_packages: Vec<TrackedAddonPackage>,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemovedAddonPackageResult {
    pub dry_run: bool,
    pub registry_path: PathBuf,
    pub removed_packages: Vec<TrackedAddonPackage>,
    pub removed_addons: Vec<String>,
    pub registry_cleaned: bool,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackedAddonPackage {
    pub package_id: String,
    pub source: AddonSourceRef,
    pub installed_at: String,
    pub updated_at: String,
    pub addons: Vec<TrackedAddon>,
    #[serde(default)]
    pub metadata: Option<AddonPackageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackedAddon {
    pub directory_name: String,
    pub toc_file: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddonPackageMetadata {
    #[serde(default)]
    pub index_name: Option<String>,
    #[serde(default)]
    pub index_package_id: Option<String>,
    #[serde(default)]
    pub package_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    #[serde(default)]
    pub supported_flavors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AddonRegistry {
    schema_version: u32,
    packages: Vec<TrackedAddonPackage>,
}

impl Default for AddonRegistry {
    fn default() -> Self {
        Self {
            schema_version: 1,
            packages: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PreparedAddonPackage {
    pub(crate) source: AddonSourceRef,
    pub(crate) package_id: String,
    pub(crate) addons: Vec<PreparedAddonDirectory>,
    pub(crate) metadata: Option<AddonPackageMetadata>,
    _stage_dir: TempDir,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedAddonDirectory {
    pub(crate) addon: TrackedAddon,
    stage_path: PathBuf,
    pub(crate) file_count: usize,
}

pub fn list_addons(installation: &DetectedFlavorInstallation) -> AppResult<AddonInventory> {
    let registry_path = registry_path(installation);
    let registry = load_registry(installation)?;
    let tracked_addons = registry
        .packages
        .iter()
        .flat_map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.directory_name.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut untracked_addons = discover_addon_directories(&installation.addon_dir)?
        .into_iter()
        .filter(|name| !tracked_addons.contains(name))
        .collect::<Vec<_>>();
    untracked_addons.sort();

    Ok(AddonInventory {
        target_addon_root: installation.addon_dir.clone(),
        registry_path,
        tracked_packages: registry.packages,
        untracked_addons,
    })
}

pub fn search_addons(request: SearchAddonRequest) -> AppResult<AddonSearchCatalog> {
    let provider = DefaultAddonProvider::default();
    search_addons_with_provider(&provider, request)
}

pub(crate) fn search_addons_with_provider<P>(
    provider: &P,
    request: SearchAddonRequest,
) -> AppResult<AddonSearchCatalog>
where
    P: AddonProvider + ?Sized,
{
    let results = provider.search_addons(ProviderAddonSearchRequest {
        query: &request.query,
        flavor: request.installation.flavor,
        limit: request.limit,
    })?;
    Ok(AddonSearchCatalog {
        query: request.query,
        results,
    })
}

fn discover_addon_directories(addon_dir: &Path) -> AppResult<Vec<String>> {
    if !addon_dir.exists() {
        return Ok(Vec::new());
    }

    let mut addons = Vec::new();
    for entry in fs::read_dir(addon_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".hearthsync" {
            continue;
        }

        if find_primary_toc(&path, &name)?.is_some() {
            addons.push(name);
        }
    }

    addons.sort();
    Ok(addons)
}
