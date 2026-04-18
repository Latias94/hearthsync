use std::path::PathBuf;

use crate::core::bundle::BundleApplyMappings;
use crate::core::install::{DetectedFlavorInstallation, WowFlavor};

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
pub struct ListBackupsRequest {
    pub backup_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct InspectBundleRequest {
    pub bundle_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PlanBundleApplyRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone)]
pub struct PlanBundleAddonLockRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
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
