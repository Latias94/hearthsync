use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::{HealthStatusValue, ResolvedInstallationValue, WowFlavorValue};
use crate::core::install::{
    DetectedFlavorInstallation, InstallationHealth, ProductInstallInspection,
};

#[derive(Debug, Clone, Serialize)]
pub struct InstallationScanResult {
    pub installation_count: usize,
    pub installations: Vec<ResolvedInstallationValue>,
}

impl InstallationScanResult {
    pub(crate) fn from_installations(installations: Vec<DetectedFlavorInstallation>) -> Self {
        let installation_count = installations.len();
        let installations = installations
            .into_iter()
            .map(ResolvedInstallationValue::from_domain)
            .collect();

        Self {
            installation_count,
            installations,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallationHealthResult {
    pub status: HealthStatusValue,
    pub status_label: String,
    pub missing_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

impl InstallationHealthResult {
    pub(crate) fn from_domain(value: InstallationHealth) -> Self {
        let status_label = value.summary().to_string();

        Self {
            status: HealthStatusValue::from_domain(value.status),
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
    pub available_flavors: Vec<WowFlavorValue>,
    pub installation: ResolvedInstallationValue,
    pub health: InstallationHealthResult,
}

impl InstallationInspectionResult {
    pub(crate) fn from_domain(value: ProductInstallInspection) -> Self {
        Self {
            requested_path: value.requested_path,
            product_root: value.product_root,
            available_flavors: value
                .available_flavors
                .into_iter()
                .map(WowFlavorValue::from_domain)
                .collect(),
            installation: ResolvedInstallationValue::from_domain(value.installation),
            health: InstallationHealthResult::from_domain(value.health),
        }
    }
}
