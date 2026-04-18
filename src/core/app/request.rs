use std::path::PathBuf;

use crate::core::addon::index::{
    AddonIndexInstallRequest as DomainAddonIndexInstallRequest,
    AddonIndexUpdateRequest as DomainAddonIndexUpdateRequest,
};
use crate::core::addon::lock::{
    AddonLockApplyRequest as DomainAddonLockApplyRequest,
    AddonLockSourceOverride as DomainAddonLockSourceOverride,
};
use crate::core::addon::{
    AddonPackageMetadata, InstallAddonRequest as DomainInstallAddonRequest,
    RemoveAddonRequest as DomainRemoveAddonRequest, SearchAddonRequest as DomainSearchAddonRequest,
    UpdateAddonRequest as DomainUpdateAddonRequest,
};
use crate::core::backup::{
    BackupGroup, BackupRequest as DomainBackupRequest,
    RestoreBackupRequest as DomainRestoreBackupRequest,
};
use crate::core::bundle::{
    AnalyzeExternalPackageRequest as DomainAnalyzeExternalPackageRequest,
    ApplyExternalPackageRequest as DomainApplyExternalPackageRequest,
    BundleAddonLockApplyRequest as DomainBundleAddonLockApplyRequest, BundleApplyMappings,
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PackBundleRequest as DomainPackBundleRequest,
    PlanExternalPackageApplyRequest as DomainPlanExternalPackageApplyRequest,
    UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::manifest::{ApplyDefaults, BundleManifest};

#[derive(Debug, Clone)]
pub struct SearchAddonsRequest {
    pub installation: DetectedFlavorInstallation,
    pub query: String,
    pub limit: usize,
}

impl From<SearchAddonsRequest> for DomainSearchAddonRequest {
    fn from(request: SearchAddonsRequest) -> Self {
        Self {
            installation: request.installation,
            query: request.query,
            limit: request.limit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListAddonsRequest {
    pub installation: DetectedFlavorInstallation,
}

#[derive(Debug, Clone)]
pub struct InspectAddonIndexRequest {
    pub index_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InspectAddonLockRequest {
    pub installation: DetectedFlavorInstallation,
}

#[derive(Debug, Clone)]
pub struct WriteAddonLockRequest {
    pub installation: DetectedFlavorInstallation,
}

#[derive(Debug, Clone)]
pub struct DiffAddonLockRequest {
    pub left_lock_path: PathBuf,
    pub right_lock_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct VerifyAddonLockRequest {
    pub installation: DetectedFlavorInstallation,
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct PlanAddonLockSyncRequest {
    pub installation: DetectedFlavorInstallation,
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AddonLockSourceOverrideRequest {
    pub comparison_key: String,
    pub archive_path: PathBuf,
}

impl From<AddonLockSourceOverrideRequest> for DomainAddonLockSourceOverride {
    fn from(request: AddonLockSourceOverrideRequest) -> Self {
        Self {
            comparison_key: request.comparison_key,
            archive_path: request.archive_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplyAddonLockAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub lock_path: Option<PathBuf>,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub source_overrides: Vec<AddonLockSourceOverrideRequest>,
}

impl From<ApplyAddonLockAppRequest> for DomainAddonLockApplyRequest {
    fn from(request: ApplyAddonLockAppRequest) -> Self {
        Self {
            installation: request.installation,
            lock_path: request.lock_path,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            source_overrides: request
                .source_overrides
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallAddonAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub source: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub metadata: Option<AddonPackageMetadata>,
}

impl From<InstallAddonAppRequest> for DomainInstallAddonRequest {
    fn from(request: InstallAddonAppRequest) -> Self {
        Self {
            installation: request.installation,
            source: request.source,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            metadata: request.metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl From<UpdateAddonAppRequest> for DomainUpdateAddonRequest {
    fn from(request: UpdateAddonAppRequest) -> Self {
        Self {
            installation: request.installation,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoveAddonAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl From<RemoveAddonAppRequest> for DomainRemoveAddonRequest {
    fn from(request: RemoveAddonAppRequest) -> Self {
        Self {
            installation: request.installation,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallAddonIndexAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub index_path: PathBuf,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl From<InstallAddonIndexAppRequest> for DomainAddonIndexInstallRequest {
    fn from(request: InstallAddonIndexAppRequest) -> Self {
        Self {
            installation: request.installation,
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonIndexAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl From<UpdateAddonIndexAppRequest> for DomainAddonIndexUpdateRequest {
    fn from(request: UpdateAddonIndexAppRequest) -> Self {
        Self {
            installation: request.installation,
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListBackupsRequest {
    pub backup_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CreateBackupAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub output_path: Option<PathBuf>,
    pub groups: Vec<BackupGroup>,
    pub label: Option<String>,
}

impl From<CreateBackupAppRequest> for DomainBackupRequest {
    fn from(request: CreateBackupAppRequest) -> Self {
        Self {
            installation: request.installation,
            output_path: request.output_path,
            groups: request.groups,
            label: request.label,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RestoreBackupAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub archive_path: Option<PathBuf>,
    pub backup_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
}

impl From<RestoreBackupAppRequest> for DomainRestoreBackupRequest {
    fn from(request: RestoreBackupAppRequest) -> Self {
        Self {
            installation: request.installation,
            archive_path: request.archive_path,
            backup_id: request.backup_id,
            backup_dir: request.backup_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InspectBundleRequest {
    pub bundle_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PackBundleAppRequest {
    pub installation: DetectedFlavorInstallation,
    pub manifest: BundleManifest,
    pub output_path: Option<PathBuf>,
    pub manifest_base_dir: Option<PathBuf>,
}

impl From<PackBundleAppRequest> for DomainPackBundleRequest {
    fn from(request: PackBundleAppRequest) -> Self {
        Self {
            installation: request.installation,
            manifest: request.manifest,
            output_path: request.output_path,
            manifest_base_dir: request.manifest_base_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanBundleApplyRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone)]
pub struct ApplyBundleAppRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappings,
}

impl From<ApplyBundleAppRequest> for DomainUnpackBundleRequest {
    fn from(request: ApplyBundleAppRequest) -> Self {
        Self {
            bundle_path: request.bundle_path,
            installation: request.installation,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            apply_mappings: request.apply_mappings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanBundleAddonLockRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
}

#[derive(Debug, Clone)]
pub struct ApplyBundleAddonLockAppRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl From<ApplyBundleAddonLockAppRequest> for DomainBundleAddonLockApplyRequest {
    fn from(request: ApplyBundleAddonLockAppRequest) -> Self {
        Self {
            bundle_path: request.bundle_path,
            installation: request.installation,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalyzeExternalPackageAppRequest {
    pub source_path: PathBuf,
}

impl From<AnalyzeExternalPackageAppRequest> for DomainAnalyzeExternalPackageRequest {
    fn from(request: AnalyzeExternalPackageAppRequest) -> Self {
        Self {
            source_path: request.source_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateExternalPackageBundleAppRequest {
    pub source_path: PathBuf,
    pub source_flavor: WowFlavor,
    pub source_platform: Option<HostPlatform>,
    pub supported_targets: Vec<WowFlavor>,
    pub output_path: Option<PathBuf>,
    pub package_id: Option<String>,
    pub package_name: Option<String>,
    pub created_by: Option<String>,
    pub description: Option<String>,
    pub apply_defaults: Option<ApplyDefaults>,
}

impl From<CreateExternalPackageBundleAppRequest> for DomainCreateExternalPackageBundleRequest {
    fn from(request: CreateExternalPackageBundleAppRequest) -> Self {
        Self {
            source_path: request.source_path,
            source_flavor: request.source_flavor,
            source_platform: request.source_platform,
            supported_targets: request.supported_targets,
            output_path: request.output_path,
            package_id: request.package_id,
            package_name: request.package_name,
            created_by: request.created_by,
            description: request.description,
            apply_defaults: request.apply_defaults,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanExternalPackageApplyAppRequest {
    pub external_package: CreateExternalPackageBundleAppRequest,
    pub installation: DetectedFlavorInstallation,
    pub apply_mappings: BundleApplyMappings,
}

impl From<PlanExternalPackageApplyAppRequest> for DomainPlanExternalPackageApplyRequest {
    fn from(request: PlanExternalPackageApplyAppRequest) -> Self {
        Self {
            external_package: request.external_package.into(),
            installation: request.installation,
            apply_mappings: request.apply_mappings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplyExternalPackageAppRequest {
    pub external_package: CreateExternalPackageBundleAppRequest,
    pub installation: DetectedFlavorInstallation,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappings,
}

impl From<ApplyExternalPackageAppRequest> for DomainApplyExternalPackageRequest {
    fn from(request: ApplyExternalPackageAppRequest) -> Self {
        Self {
            external_package: request.external_package.into(),
            installation: request.installation,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            apply_mappings: request.apply_mappings,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InspectInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavor>,
}

#[derive(Debug, Clone)]
pub struct ResolveInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavor>,
}
