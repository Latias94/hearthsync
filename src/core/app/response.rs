use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::core::addon::index::{
    AddonIndexInspection, AddonIndexInstallResult as DomainAddonIndexInstallResult,
    AddonIndexPackage, AddonIndexUpdateResult as DomainAddonIndexUpdateResult,
};
use crate::core::addon::lock::{
    AddonLockApplyResult as DomainAddonLockApplyResult,
    AddonLockDiffResult as DomainAddonLockDiffResult,
    AddonLockFieldChange as DomainAddonLockFieldChange, AddonLockInspection, AddonLockPackage,
    AddonLockPackageDiff as DomainAddonLockPackageDiff,
    AddonLockPackageDirectoryIssue as DomainAddonLockPackageDirectoryIssue,
    AddonLockPackageSnapshot as DomainAddonLockPackageSnapshot,
    AddonLockPlanResult as DomainAddonLockPlanResult,
    AddonLockSyncAction as DomainAddonLockSyncAction, AddonLockSyncActionKind,
    AddonLockVerifyResult as DomainAddonLockVerifyResult,
    AddonLockWriteResult as DomainAddonLockWriteResult,
};
use crate::core::addon::{
    AddonInventory, AddonPackageMetadata, AddonSearchCatalog as DomainAddonSearchCatalog,
    AddonSearchResult as DomainAddonSearchResult, AddonSourceRef as DomainAddonSourceRef,
    InstalledAddonPackageResult as DomainInstalledAddonPackageResult,
    RemovedAddonPackageResult as DomainRemovedAddonPackageResult, TrackedAddon,
    TrackedAddonPackage, UpdatedAddonPackageResult as DomainUpdatedAddonPackageResult,
};
use crate::core::backup::{
    BackupCatalog, BackupCatalogEntry, BackupGroup, BackupMetadata,
    CreatedBackup as DomainCreatedBackup, RestoredBackup as DomainRestoredBackup,
};
use crate::core::bundle::{
    AppliedExternalPackage as DomainAppliedExternalPackage, ApplyAction, ApplyGroup,
    ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary,
    BundleAddonLockApply as DomainBundleAddonLockApply,
    BundleAddonLockPlan as DomainBundleAddonLockPlan, BundleApplyPlan as DomainBundleApplyPlan,
    BundleEntryCounts, BundleInspection, CreatedBundle as DomainCreatedBundle,
    ExternalPackageAnalysis as DomainExternalPackageAnalysis,
    ExternalPackageApplyPlan as DomainExternalPackageApplyPlan,
    ExternalPackageEntry as DomainExternalPackageEntry, ExternalPackageSourceKind,
    ExternalPackageSummary as DomainExternalPackageSummary,
    ExternalPackageWarning as DomainExternalPackageWarning, ExternalPackageWarningCategory,
    ExternalPackageWarningCode, ExternalPackageWarningGroup as DomainExternalPackageWarningGroup,
    GroupPolicy, HelperStrategy,
    PreparedExternalPackageBundle as DomainPreparedExternalPackageBundle,
    UnpackedBundle as DomainUnpackedBundle, WtfScope,
};
use crate::core::install::{
    DetectedFlavorInstallation, HealthStatus, InstallationHealth, LocalWowAccount,
    LocalWowCharacter, ProductInstallInspection, WowFlavor,
};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};

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
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

impl From<DomainAddonSourceRef> for AddonSourceResult {
    fn from(value: DomainAddonSourceRef) -> Self {
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
    pub source: AddonSourceResult,
    pub source_label: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub metadata: Option<AddonMetadataResult>,
}

impl From<TrackedAddonPackage> for TrackedAddonPackageResult {
    fn from(value: TrackedAddonPackage) -> Self {
        let source = AddonSourceResult::from(value.source);
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

impl From<DomainAddonSearchResult> for AddonSearchResult {
    fn from(value: DomainAddonSearchResult) -> Self {
        let source = AddonSourceResult::from(value.source);
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

impl From<DomainAddonSearchCatalog> for AddonSearchCatalogResult {
    fn from(value: DomainAddonSearchCatalog) -> Self {
        let result_count = value.results.len();

        Self {
            query: value.query,
            result_count,
            results: value
                .results
                .into_iter()
                .map(AddonSearchResult::from)
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

impl From<DomainInstalledAddonPackageResult> for InstalledAddonPackageResult {
    fn from(value: DomainInstalledAddonPackageResult) -> Self {
        let source = AddonSourceResult::from(value.source);
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
                .map(TrackedAddonResult::from)
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

impl From<DomainUpdatedAddonPackageResult> for UpdatedAddonPackageResult {
    fn from(value: DomainUpdatedAddonPackageResult) -> Self {
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
                .map(TrackedAddonPackageResult::from)
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

impl From<DomainRemovedAddonPackageResult> for RemovedAddonPackageResult {
    fn from(value: DomainRemovedAddonPackageResult) -> Self {
        let removed_package_count = value.removed_packages.len();
        let removed_addon_count = value.removed_addons.len();

        Self {
            dry_run: value.dry_run,
            registry_path: value.registry_path,
            removed_package_count,
            removed_packages: value
                .removed_packages
                .into_iter()
                .map(TrackedAddonPackageResult::from)
                .collect(),
            removed_addon_count,
            removed_addons: value.removed_addons,
            registry_cleaned: value.registry_cleaned,
            backup_path: value.backup_path,
        }
    }
}

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

impl From<AddonIndexPackage> for AddonIndexPackageResult {
    fn from(value: AddonIndexPackage) -> Self {
        let source = AddonSourceResult::from(value.source);
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
pub struct AddonIndexInstallResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackageResult,
    pub install: InstalledAddonPackageResult,
}

impl From<DomainAddonIndexInstallResult> for AddonIndexInstallResult {
    fn from(value: DomainAddonIndexInstallResult) -> Self {
        Self {
            index_path: value.index_path,
            package: AddonIndexPackageResult::from(value.package),
            install: InstalledAddonPackageResult::from(value.install),
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

impl From<DomainAddonIndexUpdateResult> for AddonIndexUpdateResult {
    fn from(value: DomainAddonIndexUpdateResult) -> Self {
        let selected_package_count = value.selected_packages.len();

        Self {
            index_path: value.index_path,
            selected_package_count,
            selected_packages: value
                .selected_packages
                .into_iter()
                .map(AddonIndexPackageResult::from)
                .collect(),
            update: UpdatedAddonPackageResult::from(value.update),
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
    pub source: AddonSourceResult,
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
        let source = AddonSourceResult::from(value.source);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            name: value.name,
            version: value.version,
            source,
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
pub struct AddonLockWriteResult {
    pub lock_path: PathBuf,
    pub package_count: usize,
    pub removed: bool,
}

impl From<DomainAddonLockWriteResult> for AddonLockWriteResult {
    fn from(value: DomainAddonLockWriteResult) -> Self {
        Self {
            lock_path: value.lock_path,
            package_count: value.package_count,
            removed: value.removed,
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
pub struct BackupMetadataResult {
    pub schema_version: u32,
    pub created_at: String,
    pub label: Option<String>,
    pub flavor: String,
    pub flavor_root: PathBuf,
    pub group_count: usize,
    pub groups: Vec<BackupGroup>,
}

impl From<BackupMetadata> for BackupMetadataResult {
    fn from(value: BackupMetadata) -> Self {
        let group_count = value.groups.len();

        Self {
            schema_version: value.schema_version,
            created_at: value.created_at,
            label: value.label,
            flavor: value.flavor,
            flavor_root: value.flavor_root,
            group_count,
            groups: value.groups,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBackupResult {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub metadata: BackupMetadataResult,
}

impl From<DomainCreatedBackup> for CreatedBackupResult {
    fn from(value: DomainCreatedBackup) -> Self {
        Self {
            archive_path: value.archive_path,
            archived_files: value.archived_files,
            metadata: BackupMetadataResult::from(value.metadata),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoredBackupResult {
    pub archive_path: PathBuf,
    pub restored_files: usize,
    pub metadata: BackupMetadataResult,
}

impl From<DomainRestoredBackup> for RestoredBackupResult {
    fn from(value: DomainRestoredBackup) -> Self {
        Self {
            archive_path: value.archive_path,
            restored_files: value.restored_files,
            metadata: BackupMetadataResult::from(value.metadata),
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

impl From<BundleResources> for BundleResourcesResult {
    fn from(value: BundleResources) -> Self {
        let addon_count = value.addons.len();
        let wtf_character_count = value.wtf_characters.len();
        let interface_asset_count = value.interface_assets.len();

        Self {
            addons: value.addons,
            addon_count,
            wtf_common: value.wtf_common,
            wtf_character_count,
            wtf_characters: value
                .wtf_characters
                .into_iter()
                .map(BundleCharacterResourceResult::from)
                .collect(),
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            interface_asset_count,
            addon_lock: value.addon_lock,
            addon_indexes: value.addon_indexes,
        }
    }
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
        let package = BundlePackageResult::from(value.manifest.package);
        let source = BundleSourceResult::from(value.manifest.source);
        let resources = BundleResourcesResult::from(value.manifest.resources);

        Self {
            archive_path: value.archive_path,
            package,
            source,
            resources,
            entries: BundleEntryCountsResult::from(value.entries),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBundleResult {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub manifest: BundleManifestResult,
}

impl From<DomainCreatedBundle> for CreatedBundleResult {
    fn from(value: DomainCreatedBundle) -> Self {
        Self {
            archive_path: value.archive_path,
            archived_files: value.archived_files,
            manifest: BundleManifestResult::from(value.manifest),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageBundleResult {
    pub analysis: ExternalPackageAnalysisResult,
    pub manifest: BundleManifestResult,
    pub bundle: CreatedBundleResult,
}

#[derive(Debug)]
pub struct ExternalPackageBundleHandle {
    result: ExternalPackageBundleResult,
    _prepared: DomainPreparedExternalPackageBundle,
}

impl ExternalPackageBundleHandle {
    pub fn result(&self) -> &ExternalPackageBundleResult {
        &self.result
    }

    pub fn analysis(&self) -> &ExternalPackageAnalysisResult {
        &self.result.analysis
    }

    pub fn manifest(&self) -> &BundleManifestResult {
        &self.result.manifest
    }

    pub fn bundle(&self) -> &CreatedBundleResult {
        &self.result.bundle
    }

    pub fn archive_path(&self) -> &Path {
        &self.result.bundle.archive_path
    }
}

impl From<DomainPreparedExternalPackageBundle> for ExternalPackageBundleHandle {
    fn from(value: DomainPreparedExternalPackageBundle) -> Self {
        let result = ExternalPackageBundleResult {
            analysis: ExternalPackageAnalysisResult::from(value.analysis.clone()),
            manifest: BundleManifestResult::from(value.manifest.clone()),
            bundle: CreatedBundleResult::from(value.bundle.clone()),
        };

        Self {
            result,
            _prepared: value,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowCharacterResult {
    pub server: String,
    pub character: String,
    pub character_dir: PathBuf,
}

impl From<LocalWowCharacter> for LocalWowCharacterResult {
    fn from(value: LocalWowCharacter) -> Self {
        Self {
            server: value.server,
            character: value.character,
            character_dir: value.character_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowAccountResult {
    pub account_name: String,
    pub account_dir: PathBuf,
    pub saved_variables_dir: PathBuf,
    pub characters: Vec<LocalWowCharacterResult>,
}

impl From<LocalWowAccount> for LocalWowAccountResult {
    fn from(value: LocalWowAccount) -> Self {
        Self {
            account_name: value.account_name,
            account_dir: value.account_dir,
            saved_variables_dir: value.saved_variables_dir,
            characters: value
                .characters
                .into_iter()
                .map(LocalWowCharacterResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CharacterMappingResult {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: String,
    pub target_server: String,
    pub target_character: String,
}

impl From<CharacterMapping> for CharacterMappingResult {
    fn from(value: CharacterMapping) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOperationResult {
    pub group: ApplyGroup,
    pub wtf_scope: Option<WtfScope>,
    pub action: ApplyAction,
    pub archive_name: String,
    pub destination: PathBuf,
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
}

impl From<ApplyOperation> for ApplyOperationResult {
    fn from(value: ApplyOperation) -> Self {
        Self {
            group: value.group,
            wtf_scope: value.wtf_scope,
            action: value.action,
            archive_name: value.archive_name,
            destination: value.destination,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyPlanSummaryResult {
    pub files_to_add: usize,
    pub files_to_replace: usize,
    pub files_to_skip: usize,
    pub paths_to_remove: usize,
    pub files_to_preserve: usize,
}

impl From<ApplyPlanSummary> for ApplyPlanSummaryResult {
    fn from(value: ApplyPlanSummary) -> Self {
        Self {
            files_to_add: value.files_to_add,
            files_to_replace: value.files_to_replace,
            files_to_skip: value.files_to_skip,
            paths_to_remove: value.paths_to_remove,
            files_to_preserve: value.files_to_preserve,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupPolicyResult {
    pub policy: ResourceApplyPolicy,
}

impl From<GroupPolicy> for GroupPolicyResult {
    fn from(value: GroupPolicy) -> Self {
        Self {
            policy: value.policy,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyGroupPoliciesResult {
    pub addons: GroupPolicyResult,
    pub wtf_common: GroupPolicyResult,
    pub wtf_characters: GroupPolicyResult,
    pub fonts: GroupPolicyResult,
    pub interface_assets: GroupPolicyResult,
    pub metadata: GroupPolicyResult,
}

impl From<ApplyGroupPolicies> for ApplyGroupPoliciesResult {
    fn from(value: ApplyGroupPolicies) -> Self {
        Self {
            addons: GroupPolicyResult::from(value.addons),
            wtf_common: GroupPolicyResult::from(value.wtf_common),
            wtf_characters: GroupPolicyResult::from(value.wtf_characters),
            fonts: GroupPolicyResult::from(value.fonts),
            interface_assets: GroupPolicyResult::from(value.interface_assets),
            metadata: GroupPolicyResult::from(value.metadata),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleMappingRulesResult {
    pub character_mode: CharacterMappingMode,
    pub rewrite_profile_keys: bool,
    pub rewrite_identity_strings: bool,
    pub allow_cross_platform: bool,
}

impl From<MappingRules> for BundleMappingRulesResult {
    fn from(value: MappingRules) -> Self {
        Self {
            character_mode: value.character_mode,
            rewrite_profile_keys: value.rewrite_profile_keys,
            rewrite_identity_strings: value.rewrite_identity_strings,
            allow_cross_platform: value.allow_cross_platform,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleApplyDefaultsResult {
    pub create_backup: bool,
    pub addons: ResourceApplyPolicy,
    pub wtf_common: ResourceApplyPolicy,
    pub wtf_characters: ResourceApplyPolicy,
    pub fonts: ResourceApplyPolicy,
    pub interface_assets: ResourceApplyPolicy,
}

impl From<ApplyDefaults> for BundleApplyDefaultsResult {
    fn from(value: ApplyDefaults) -> Self {
        Self {
            create_backup: value.create_backup,
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters,
            fonts: value.fonts,
            interface_assets: value.interface_assets,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleManifestResult {
    pub schema_version: u32,
    pub package: BundlePackageResult,
    pub source: BundleSourceResult,
    pub resources: BundleResourcesResult,
    pub mapping: BundleMappingRulesResult,
    pub apply: BundleApplyDefaultsResult,
}

impl From<BundleManifest> for BundleManifestResult {
    fn from(value: BundleManifest) -> Self {
        Self {
            schema_version: value.schema_version,
            package: BundlePackageResult::from(value.package),
            source: BundleSourceResult::from(value.source),
            resources: BundleResourcesResult::from(value.resources),
            mapping: BundleMappingRulesResult::from(value.mapping),
            apply: BundleApplyDefaultsResult::from(value.apply),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleApplyPlanResult {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<LocalWowAccountResult>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<CharacterMappingResult>,
    pub operations: Vec<ApplyOperationResult>,
    pub summary: ApplyPlanSummaryResult,
    pub helper_strategy: HelperStrategy,
    pub group_policies: ApplyGroupPoliciesResult,
    pub manifest: BundleManifestResult,
}

impl From<DomainBundleApplyPlan> for BundleApplyPlanResult {
    fn from(value: DomainBundleApplyPlan) -> Self {
        Self {
            bundle_path: value.bundle_path,
            target_flavor_root: value.target_flavor_root,
            discovered_accounts: value
                .discovered_accounts
                .into_iter()
                .map(LocalWowAccountResult::from)
                .collect(),
            selected_target_accounts: value.selected_target_accounts,
            character_mappings: value
                .character_mappings
                .into_iter()
                .map(CharacterMappingResult::from)
                .collect(),
            operations: value
                .operations
                .into_iter()
                .map(ApplyOperationResult::from)
                .collect(),
            summary: ApplyPlanSummaryResult::from(value.summary),
            helper_strategy: value.helper_strategy,
            group_policies: ApplyGroupPoliciesResult::from(value.group_policies),
            manifest: BundleManifestResult::from(value.manifest),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleApplyResult {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub dry_run: bool,
    pub planned_files: usize,
    pub written_files: usize,
    pub rewritten_files: usize,
    pub backup_path: Option<PathBuf>,
    pub selected_target_accounts: Vec<String>,
    pub plan_summary: ApplyPlanSummaryResult,
    pub character_mappings: Vec<CharacterMappingResult>,
    pub manifest: BundleManifestResult,
}

impl From<DomainUnpackedBundle> for BundleApplyResult {
    fn from(value: DomainUnpackedBundle) -> Self {
        Self {
            bundle_path: value.bundle_path,
            target_flavor_root: value.target_flavor_root,
            dry_run: value.dry_run,
            planned_files: value.planned_files,
            written_files: value.written_files,
            rewritten_files: value.rewritten_files,
            backup_path: value.backup_path,
            selected_target_accounts: value.selected_target_accounts,
            plan_summary: ApplyPlanSummaryResult::from(value.plan_summary),
            character_mappings: value
                .character_mappings
                .into_iter()
                .map(CharacterMappingResult::from)
                .collect(),
            manifest: BundleManifestResult::from(value.manifest),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageSnapshotResult {
    pub comparison_key: String,
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: Option<String>,
    pub addon_directories: Vec<String>,
}

impl From<DomainAddonLockPackageSnapshot> for AddonLockPackageSnapshotResult {
    fn from(value: DomainAddonLockPackageSnapshot) -> Self {
        let source = AddonSourceResult::from(value.source);
        let source_label = source.display_name.clone();

        Self {
            comparison_key: value.comparison_key,
            package_id: value.package_id,
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            name: value.name,
            version: value.version,
            source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            content_sha256: value.content_sha256,
            addon_directories: value.addon_directories,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockFieldChangeResult {
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

impl From<DomainAddonLockFieldChange> for AddonLockFieldChangeResult {
    fn from(value: DomainAddonLockFieldChange) -> Self {
        Self {
            field: value.field,
            left: value.left,
            right: value.right,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDiffResult {
    pub comparison_key: String,
    pub left: AddonLockPackageSnapshotResult,
    pub right: AddonLockPackageSnapshotResult,
    pub changes: Vec<AddonLockFieldChangeResult>,
}

impl From<DomainAddonLockPackageDiff> for AddonLockPackageDiffResult {
    fn from(value: DomainAddonLockPackageDiff) -> Self {
        Self {
            comparison_key: value.comparison_key,
            left: AddonLockPackageSnapshotResult::from(value.left),
            right: AddonLockPackageSnapshotResult::from(value.right),
            changes: value
                .changes
                .into_iter()
                .map(AddonLockFieldChangeResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockDiffResult {
    pub left_label: String,
    pub right_label: String,
    pub left_package_count: usize,
    pub right_package_count: usize,
    pub identical: bool,
    pub unchanged_packages: usize,
    pub added_package_count: usize,
    pub removed_package_count: usize,
    pub changed_package_count: usize,
    pub added_packages: Vec<AddonLockPackageSnapshotResult>,
    pub removed_packages: Vec<AddonLockPackageSnapshotResult>,
    pub changed_packages: Vec<AddonLockPackageDiffResult>,
}

impl From<DomainAddonLockDiffResult> for AddonLockDiffResult {
    fn from(value: DomainAddonLockDiffResult) -> Self {
        let added_package_count = value.added_packages.len();
        let removed_package_count = value.removed_packages.len();
        let changed_package_count = value.changed_packages.len();

        Self {
            left_label: value.left_label,
            right_label: value.right_label,
            left_package_count: value.left_package_count,
            right_package_count: value.right_package_count,
            identical: value.identical,
            unchanged_packages: value.unchanged_packages,
            added_package_count,
            removed_package_count,
            changed_package_count,
            added_packages: value
                .added_packages
                .into_iter()
                .map(AddonLockPackageSnapshotResult::from)
                .collect(),
            removed_packages: value
                .removed_packages
                .into_iter()
                .map(AddonLockPackageSnapshotResult::from)
                .collect(),
            changed_packages: value
                .changed_packages
                .into_iter()
                .map(AddonLockPackageDiffResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDirectoryIssueResult {
    pub comparison_key: String,
    pub package_id: String,
    pub missing_addon_directories: Vec<String>,
}

impl From<DomainAddonLockPackageDirectoryIssue> for AddonLockPackageDirectoryIssueResult {
    fn from(value: DomainAddonLockPackageDirectoryIssue) -> Self {
        Self {
            comparison_key: value.comparison_key,
            package_id: value.package_id,
            missing_addon_directories: value.missing_addon_directories,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockVerifyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub tracked_package_count: usize,
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub missing_package_count: usize,
    pub missing_addon_directories: Vec<AddonLockPackageDirectoryIssueResult>,
    pub diff: AddonLockDiffResult,
    pub matches: bool,
}

impl From<DomainAddonLockVerifyResult> for AddonLockVerifyResult {
    fn from(value: DomainAddonLockVerifyResult) -> Self {
        let untracked_addon_count = value.untracked_addons.len();
        let missing_package_count = value.missing_addon_directories.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            tracked_package_count: value.tracked_package_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            missing_package_count,
            missing_addon_directories: value
                .missing_addon_directories
                .into_iter()
                .map(AddonLockPackageDirectoryIssueResult::from)
                .collect(),
            diff: AddonLockDiffResult::from(value.diff),
            matches: value.matches,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockSyncActionResult {
    pub kind: AddonLockSyncActionKind,
    pub comparison_key: String,
    pub package_id: String,
    pub name: Option<String>,
    pub addon_directories: Vec<String>,
    pub source: Option<AddonSourceResult>,
    pub source_label: Option<String>,
    pub reasons: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub requires_replace_existing: bool,
}

impl From<DomainAddonLockSyncAction> for AddonLockSyncActionResult {
    fn from(value: DomainAddonLockSyncAction) -> Self {
        let source = value.source.map(AddonSourceResult::from);
        let source_label = source.as_ref().map(|source| source.display_name.clone());

        Self {
            kind: value.kind,
            comparison_key: value.comparison_key,
            package_id: value.package_id,
            name: value.name,
            addon_directories: value.addon_directories,
            source,
            source_label,
            reasons: value.reasons,
            blocked_reasons: value.blocked_reasons,
            requires_replace_existing: value.requires_replace_existing,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPlanResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub action_count: usize,
    pub actions: Vec<AddonLockSyncActionResult>,
}

impl From<DomainAddonLockPlanResult> for AddonLockPlanResult {
    fn from(value: DomainAddonLockPlanResult) -> Self {
        let untracked_addon_count = value.untracked_addons.len();
        let action_count = value.actions.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            install_count: value.install_count,
            update_count: value.update_count,
            remove_count: value.remove_count,
            metadata_only_count: value.metadata_only_count,
            unchanged_count: value.unchanged_count,
            blocked_count: value.blocked_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            action_count,
            actions: value
                .actions
                .into_iter()
                .map(AddonLockSyncActionResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockApplyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub action_count: usize,
    pub actions: Vec<AddonLockSyncActionResult>,
    pub verification: AddonLockVerifyResult,
}

impl From<DomainAddonLockApplyResult> for AddonLockApplyResult {
    fn from(value: DomainAddonLockApplyResult) -> Self {
        let untracked_addon_count = value.untracked_addons.len();
        let action_count = value.actions.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            install_count: value.install_count,
            update_count: value.update_count,
            remove_count: value.remove_count,
            metadata_only_count: value.metadata_only_count,
            unchanged_count: value.unchanged_count,
            blocked_count: value.blocked_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            action_count,
            actions: value
                .actions
                .into_iter()
                .map(AddonLockSyncActionResult::from)
                .collect(),
            verification: AddonLockVerifyResult::from(value.verification),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockPlanResult {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub plan: AddonLockPlanResult,
}

impl From<DomainBundleAddonLockPlan> for BundleAddonLockPlanResult {
    fn from(value: DomainBundleAddonLockPlan) -> Self {
        Self {
            bundle_path: value.bundle_path,
            embedded_lock_entry: value.embedded_lock_entry,
            plan: AddonLockPlanResult::from(value.plan),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockApplyResult {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub apply: AddonLockApplyResult,
}

impl From<DomainBundleAddonLockApply> for BundleAddonLockApplyResult {
    fn from(value: DomainBundleAddonLockApply) -> Self {
        Self {
            bundle_path: value.bundle_path,
            embedded_lock_entry: value.embedded_lock_entry,
            apply: AddonLockApplyResult::from(value.apply),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageEntryResult {
    pub source_path: String,
    pub normalized_path: String,
    pub group: ApplyGroup,
    pub wtf_scope: Option<WtfScope>,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

impl From<DomainExternalPackageEntry> for ExternalPackageEntryResult {
    fn from(value: DomainExternalPackageEntry) -> Self {
        Self {
            source_path: value.source_path,
            normalized_path: value.normalized_path,
            group: value.group,
            wtf_scope: value.wtf_scope,
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageWarningGroupResult {
    pub category: ExternalPackageWarningCategory,
    pub code: ExternalPackageWarningCode,
    pub count: usize,
}

impl From<DomainExternalPackageWarningGroup> for ExternalPackageWarningGroupResult {
    fn from(value: DomainExternalPackageWarningGroup) -> Self {
        Self {
            category: value.category,
            code: value.code,
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageWarningResult {
    pub category: ExternalPackageWarningCategory,
    pub code: ExternalPackageWarningCode,
    pub source_path: String,
    pub message: String,
}

impl From<DomainExternalPackageWarning> for ExternalPackageWarningResult {
    fn from(value: DomainExternalPackageWarning) -> Self {
        Self {
            category: value.category,
            code: value.code,
            source_path: value.source_path,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageSummaryResult {
    pub total_files: usize,
    pub normalized_files: usize,
    pub ignored_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub warning_count: usize,
    pub addon_warning_count: usize,
    pub wtf_warning_count: usize,
    pub warning_groups: Vec<ExternalPackageWarningGroupResult>,
}

impl From<DomainExternalPackageSummary> for ExternalPackageSummaryResult {
    fn from(value: DomainExternalPackageSummary) -> Self {
        Self {
            total_files: value.total_files,
            normalized_files: value.normalized_files,
            ignored_files: value.ignored_files,
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters,
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            warning_count: value.warning_count,
            addon_warning_count: value.addon_warning_count,
            wtf_warning_count: value.wtf_warning_count,
            warning_groups: value
                .warning_groups
                .into_iter()
                .map(ExternalPackageWarningGroupResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageAnalysisResult {
    pub source_path: PathBuf,
    pub source_kind: ExternalPackageSourceKind,
    pub package_id: String,
    pub package_name: String,
    pub entry_count: usize,
    pub entries: Vec<ExternalPackageEntryResult>,
    pub resources: BundleResourcesResult,
    pub summary: ExternalPackageSummaryResult,
    pub warnings: Vec<ExternalPackageWarningResult>,
}

impl From<DomainExternalPackageAnalysis> for ExternalPackageAnalysisResult {
    fn from(value: DomainExternalPackageAnalysis) -> Self {
        let entry_count = value.entries.len();

        Self {
            source_path: value.source_path,
            source_kind: value.source_kind,
            package_id: value.package_id,
            package_name: value.package_name,
            entry_count,
            entries: value
                .entries
                .into_iter()
                .map(ExternalPackageEntryResult::from)
                .collect(),
            resources: BundleResourcesResult::from(value.resources),
            summary: ExternalPackageSummaryResult::from(value.summary),
            warnings: value
                .warnings
                .into_iter()
                .map(ExternalPackageWarningResult::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageApplyPlanResult {
    pub analysis: ExternalPackageAnalysisResult,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<LocalWowAccountResult>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<CharacterMappingResult>,
    pub operations: Vec<ApplyOperationResult>,
    pub summary: ApplyPlanSummaryResult,
    pub helper_strategy: HelperStrategy,
    pub group_policies: ApplyGroupPoliciesResult,
    pub manifest: BundleManifestResult,
}

impl From<DomainExternalPackageApplyPlan> for ExternalPackageApplyPlanResult {
    fn from(value: DomainExternalPackageApplyPlan) -> Self {
        Self {
            analysis: ExternalPackageAnalysisResult::from(value.analysis),
            target_flavor_root: value.target_flavor_root,
            discovered_accounts: value
                .discovered_accounts
                .into_iter()
                .map(LocalWowAccountResult::from)
                .collect(),
            selected_target_accounts: value.selected_target_accounts,
            character_mappings: value
                .character_mappings
                .into_iter()
                .map(CharacterMappingResult::from)
                .collect(),
            operations: value
                .operations
                .into_iter()
                .map(ApplyOperationResult::from)
                .collect(),
            summary: ApplyPlanSummaryResult::from(value.summary),
            helper_strategy: value.helper_strategy,
            group_policies: ApplyGroupPoliciesResult::from(value.group_policies),
            manifest: BundleManifestResult::from(value.manifest),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageApplyResult {
    pub analysis: ExternalPackageAnalysisResult,
    pub target_flavor_root: PathBuf,
    pub dry_run: bool,
    pub planned_files: usize,
    pub written_files: usize,
    pub rewritten_files: usize,
    pub backup_path: Option<PathBuf>,
    pub selected_target_accounts: Vec<String>,
    pub plan_summary: ApplyPlanSummaryResult,
    pub character_mappings: Vec<CharacterMappingResult>,
    pub manifest: BundleManifestResult,
}

impl From<DomainAppliedExternalPackage> for ExternalPackageApplyResult {
    fn from(value: DomainAppliedExternalPackage) -> Self {
        Self {
            analysis: ExternalPackageAnalysisResult::from(value.analysis),
            target_flavor_root: value.target_flavor_root,
            dry_run: value.dry_run,
            planned_files: value.planned_files,
            written_files: value.written_files,
            rewritten_files: value.rewritten_files,
            backup_path: value.backup_path,
            selected_target_accounts: value.selected_target_accounts,
            plan_summary: ApplyPlanSummaryResult::from(value.plan_summary),
            character_mappings: value
                .character_mappings
                .into_iter()
                .map(CharacterMappingResult::from)
                .collect(),
            manifest: BundleManifestResult::from(value.manifest),
        }
    }
}
