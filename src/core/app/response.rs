use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::index::{AddonIndexInspection, AddonIndexPackage};
use crate::core::addon::lock::{AddonLockInspection, AddonLockPackage};
use crate::core::addon::{
    AddonInventory, AddonPackageMetadata, AddonSourceRef, TrackedAddon, TrackedAddonPackage,
};
use crate::core::backup::{BackupCatalog, BackupCatalogEntry, BackupGroup};
use crate::core::bundle::{BundleEntryCounts, BundleInspection};
use crate::core::install::{
    DetectedFlavorInstallation, HealthStatus, InstallationHealth, ProductInstallInspection,
    WowFlavor,
};
use crate::core::manifest::{CharacterResource, PackageMetadata, SourceInstallation};

#[derive(Debug, Clone, Serialize)]
pub struct InstallationResult {
    pub platform: crate::core::install::HostPlatform,
    pub flavor: WowFlavor,
    pub product_root: PathBuf,
    pub flavor_root: PathBuf,
    pub interface_dir: PathBuf,
    pub addon_dir: PathBuf,
    pub wtf_dir: PathBuf,
    pub fonts_dir: PathBuf,
}

impl From<DetectedFlavorInstallation> for InstallationResult {
    fn from(value: DetectedFlavorInstallation) -> Self {
        Self {
            platform: value.platform,
            flavor: value.flavor,
            product_root: value.product_root,
            flavor_root: value.flavor_root,
            interface_dir: value.interface_dir,
            addon_dir: value.addon_dir,
            wtf_dir: value.wtf_dir,
            fonts_dir: value.fonts_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallationScanResult {
    pub installation_count: usize,
    pub installations: Vec<InstallationResult>,
}

impl InstallationScanResult {
    pub fn from_installations(installations: Vec<DetectedFlavorInstallation>) -> Self {
        let installation_count = installations.len();
        let installations = installations
            .into_iter()
            .map(InstallationResult::from)
            .collect();

        Self {
            installation_count,
            installations,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallationHealthResult {
    pub status: HealthStatus,
    pub status_label: String,
    pub missing_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

impl InstallationHealthResult {
    pub fn to_report(&self) -> String {
        let mut lines = vec![format!("Status: {}", self.status_label)];

        if self.missing_paths.is_empty() {
            lines.push("Missing required paths: none".to_string());
        } else {
            lines.push("Missing required paths:".to_string());
            for path in &self.missing_paths {
                lines.push(format!("- {}", path.display()));
            }
        }

        if self.warnings.is_empty() {
            lines.push("Warnings: none".to_string());
        } else {
            lines.push("Warnings:".to_string());
            for warning in &self.warnings {
                lines.push(format!("- {warning}"));
            }
        }

        lines.join("\n")
    }
}

impl From<InstallationHealth> for InstallationHealthResult {
    fn from(value: InstallationHealth) -> Self {
        let status_label = value.summary().to_string();

        Self {
            status: value.status,
            status_label,
            missing_paths: value.missing_paths,
            warnings: value.warnings,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallationInspectionResult {
    pub requested_path: PathBuf,
    pub product_root: PathBuf,
    pub available_flavors: Vec<WowFlavor>,
    pub installation: InstallationResult,
    pub health: InstallationHealthResult,
}

impl From<ProductInstallInspection> for InstallationInspectionResult {
    fn from(value: ProductInstallInspection) -> Self {
        Self {
            requested_path: value.requested_path,
            product_root: value.product_root,
            available_flavors: value.available_flavors,
            installation: InstallationResult::from(value.installation),
            health: InstallationHealthResult::from(value.health),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonMetadataResult {
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub package_name: Option<String>,
    pub version: Option<String>,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub supported_flavors: Vec<String>,
}

impl From<AddonPackageMetadata> for AddonMetadataResult {
    fn from(value: AddonPackageMetadata) -> Self {
        Self {
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            package_name: value.package_name,
            version: value.version,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            supported_flavors: value.supported_flavors,
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

impl From<TrackedAddon> for TrackedAddonResult {
    fn from(value: TrackedAddon) -> Self {
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
    pub source: AddonSourceRef,
    pub source_label: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub metadata: Option<AddonMetadataResult>,
}

impl From<TrackedAddonPackage> for TrackedAddonPackageResult {
    fn from(value: TrackedAddonPackage) -> Self {
        let source_label = value.source.display_name();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            source: value.source,
            source_label,
            installed_at: value.installed_at,
            updated_at: value.updated_at,
            addon_count,
            addons: value
                .addons
                .into_iter()
                .map(TrackedAddonResult::from)
                .collect(),
            metadata: value.metadata.map(AddonMetadataResult::from),
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

impl From<AddonInventory> for AddonInventoryResult {
    fn from(value: AddonInventory) -> Self {
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
                .map(TrackedAddonPackageResult::from)
                .collect(),
            untracked_addons: value.untracked_addons,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexPackageResult {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: AddonSourceRef,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub sha256: Option<String>,
    pub addon_directories: Vec<String>,
    pub supported_flavors: Vec<String>,
}

impl From<AddonIndexPackage> for AddonIndexPackageResult {
    fn from(value: AddonIndexPackage) -> Self {
        let source_label = value.source.display_name();

        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            source: value.source,
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

impl From<AddonIndexInspection> for AddonIndexInspectionResult {
    fn from(value: AddonIndexInspection) -> Self {
        Self {
            index_path: value.index_path,
            name: value.index.name,
            description: value.index.description,
            package_count: value.package_count,
            packages: value
                .index
                .packages
                .into_iter()
                .map(AddonIndexPackageResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageResult {
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceRef,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_directories: Vec<String>,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
}

impl From<AddonLockPackage> for AddonLockPackageResult {
    fn from(value: AddonLockPackage) -> Self {
        let source_label = value.source.display_name();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            name: value.name,
            version: value.version,
            source: value.source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            content_sha256: value.content_sha256,
            installed_at: value.installed_at,
            updated_at: value.updated_at,
            addon_directories: value.addon_directories,
            addon_count,
            addons: value
                .addons
                .into_iter()
                .map(TrackedAddonResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockInspectionResult {
    pub lock_path: PathBuf,
    pub generated_at: String,
    pub package_count: usize,
    pub packages: Vec<AddonLockPackageResult>,
}

impl From<AddonLockInspection> for AddonLockInspectionResult {
    fn from(value: AddonLockInspection) -> Self {
        Self {
            lock_path: value.lock_path,
            generated_at: value.lock.generated_at,
            package_count: value.package_count,
            packages: value
                .lock
                .packages
                .into_iter()
                .map(AddonLockPackageResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupEntryResult {
    pub backup_id: String,
    pub archive_path: PathBuf,
    pub archive_size_bytes: u64,
    pub created_at: String,
    pub label: Option<String>,
    pub flavor: String,
    pub flavor_root: PathBuf,
    pub groups: Vec<BackupGroup>,
}

impl From<BackupCatalogEntry> for BackupEntryResult {
    fn from(value: BackupCatalogEntry) -> Self {
        Self {
            backup_id: value.backup_id,
            archive_path: value.archive_path,
            archive_size_bytes: value.archive_size_bytes,
            created_at: value.metadata.created_at,
            label: value.metadata.label,
            flavor: value.metadata.flavor,
            flavor_root: value.metadata.flavor_root,
            groups: value.metadata.groups,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BackupCatalogResult {
    pub backup_dir: PathBuf,
    pub entry_count: usize,
    pub entries: Vec<BackupEntryResult>,
}

impl From<BackupCatalog> for BackupCatalogResult {
    fn from(value: BackupCatalog) -> Self {
        let entry_count = value.entries.len();

        Self {
            backup_dir: value.backup_dir,
            entry_count,
            entries: value
                .entries
                .into_iter()
                .map(BackupEntryResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundlePackageResult {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub description: Option<String>,
}

impl From<PackageMetadata> for BundlePackageResult {
    fn from(value: PackageMetadata) -> Self {
        Self {
            id: value.id,
            name: value.name,
            created_by: value.created_by,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleSourceResult {
    pub flavor: WowFlavor,
    pub platform: Option<crate::core::install::HostPlatform>,
    pub exported_at: Option<String>,
    pub supported_targets: Vec<WowFlavor>,
}

impl From<SourceInstallation> for BundleSourceResult {
    fn from(value: SourceInstallation) -> Self {
        Self {
            flavor: value.flavor,
            platform: value.platform,
            exported_at: value.exported_at,
            supported_targets: value.supported_targets,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleCharacterResourceResult {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_hint: Option<String>,
}

impl From<CharacterResource> for BundleCharacterResourceResult {
    fn from(value: CharacterResource) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_hint: value.target_hint,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleResourcesResult {
    pub addons: Vec<String>,
    pub addon_count: usize,
    pub wtf_common: bool,
    pub wtf_character_count: usize,
    pub wtf_characters: Vec<BundleCharacterResourceResult>,
    pub fonts: bool,
    pub interface_assets: Vec<String>,
    pub interface_asset_count: usize,
    pub addon_lock: bool,
    pub addon_indexes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleEntryCountsResult {
    pub total_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub metadata: usize,
}

impl From<BundleEntryCounts> for BundleEntryCountsResult {
    fn from(value: BundleEntryCounts) -> Self {
        Self {
            total_files: value.total_files,
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters,
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            metadata: value.metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleInspectionResult {
    pub archive_path: PathBuf,
    pub package: BundlePackageResult,
    pub source: BundleSourceResult,
    pub resources: BundleResourcesResult,
    pub entries: BundleEntryCountsResult,
}

impl From<BundleInspection> for BundleInspectionResult {
    fn from(value: BundleInspection) -> Self {
        let addon_count = value.manifest.resources.addons.len();
        let wtf_character_count = value.manifest.resources.wtf_characters.len();
        let interface_asset_count = value.manifest.resources.interface_assets.len();

        Self {
            archive_path: value.archive_path,
            package: BundlePackageResult::from(value.manifest.package),
            source: BundleSourceResult::from(value.manifest.source),
            resources: BundleResourcesResult {
                addons: value.manifest.resources.addons,
                addon_count,
                wtf_common: value.manifest.resources.wtf_common,
                wtf_character_count,
                wtf_characters: value
                    .manifest
                    .resources
                    .wtf_characters
                    .into_iter()
                    .map(BundleCharacterResourceResult::from)
                    .collect(),
                fonts: value.manifest.resources.fonts,
                interface_assets: value.manifest.resources.interface_assets,
                interface_asset_count,
                addon_lock: value.manifest.resources.addon_lock,
                addon_indexes: value.manifest.resources.addon_indexes,
            },
            entries: BundleEntryCountsResult::from(value.entries),
        }
    }
}
