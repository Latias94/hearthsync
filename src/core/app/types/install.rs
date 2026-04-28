use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use crate::core::install::{
    DetectedFlavorInstallation, HealthStatus as DomainHealthStatus,
    HostPlatform as DomainHostPlatform, WowFlavor as DomainWowFlavor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostPlatformValue {
    Windows,
    MacOs,
    Linux,
    Unknown,
}

impl HostPlatformValue {
    pub(crate) fn current() -> Self {
        Self::from_domain(DomainHostPlatform::current())
    }

    pub(crate) fn from_domain(value: DomainHostPlatform) -> Self {
        match value {
            DomainHostPlatform::Windows => Self::Windows,
            DomainHostPlatform::MacOs => Self::MacOs,
            DomainHostPlatform::Linux => Self::Linux,
            DomainHostPlatform::Unknown => Self::Unknown,
        }
    }

    pub(crate) fn into_domain(self) -> DomainHostPlatform {
        match self {
            Self::Windows => DomainHostPlatform::Windows,
            Self::MacOs => DomainHostPlatform::MacOs,
            Self::Linux => DomainHostPlatform::Linux,
            Self::Unknown => DomainHostPlatform::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WowFlavorValue {
    Retail,
    Classic,
    ClassicEra,
    Ptr,
    Beta,
    Xptr,
}

impl WowFlavorValue {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Retail => "retail",
            Self::Classic => "classic",
            Self::ClassicEra => "classic_era",
            Self::Ptr => "ptr",
            Self::Beta => "beta",
            Self::Xptr => "xptr",
        }
    }

    pub(crate) fn from_domain(value: DomainWowFlavor) -> Self {
        match value {
            DomainWowFlavor::Retail => Self::Retail,
            DomainWowFlavor::Classic => Self::Classic,
            DomainWowFlavor::ClassicEra => Self::ClassicEra,
            DomainWowFlavor::Ptr => Self::Ptr,
            DomainWowFlavor::Beta => Self::Beta,
            DomainWowFlavor::Xptr => Self::Xptr,
        }
    }

    pub(crate) fn into_domain(self) -> DomainWowFlavor {
        match self {
            Self::Retail => DomainWowFlavor::Retail,
            Self::Classic => DomainWowFlavor::Classic,
            Self::ClassicEra => DomainWowFlavor::ClassicEra,
            Self::Ptr => DomainWowFlavor::Ptr,
            Self::Beta => DomainWowFlavor::Beta,
            Self::Xptr => DomainWowFlavor::Xptr,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatusValue {
    Healthy,
    Warning,
    Broken,
}

impl HealthStatusValue {
    pub(crate) fn from_domain(value: DomainHealthStatus) -> Self {
        match value {
            DomainHealthStatus::Healthy => Self::Healthy,
            DomainHealthStatus::Warning => Self::Warning,
            DomainHealthStatus::Broken => Self::Broken,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn into_domain(self) -> DomainHealthStatus {
        match self {
            Self::Healthy => DomainHealthStatus::Healthy,
            Self::Warning => DomainHealthStatus::Warning,
            Self::Broken => DomainHealthStatus::Broken,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedInstallationValue {
    pub platform: HostPlatformValue,
    pub flavor: WowFlavorValue,
    pub product_root: PathBuf,
    pub flavor_root: PathBuf,
    pub interface_dir: PathBuf,
    pub addon_dir: PathBuf,
    pub wtf_dir: PathBuf,
    pub fonts_dir: PathBuf,
}

impl ResolvedInstallationValue {
    pub(crate) fn from_domain(value: DetectedFlavorInstallation) -> Self {
        Self {
            platform: HostPlatformValue::from_domain(value.platform),
            flavor: WowFlavorValue::from_domain(value.flavor),
            product_root: value.product_root,
            flavor_root: value.flavor_root,
            interface_dir: value.interface_dir,
            addon_dir: value.addon_dir,
            wtf_dir: value.wtf_dir,
            fonts_dir: value.fonts_dir,
        }
    }

    pub(crate) fn into_domain(self) -> AppResult<DetectedFlavorInstallation> {
        self.validate()?;
        Ok(DetectedFlavorInstallation {
            platform: self.platform.into_domain(),
            flavor: self.flavor.into_domain(),
            product_root: self.product_root,
            flavor_root: self.flavor_root,
            interface_dir: self.interface_dir,
            addon_dir: self.addon_dir,
            wtf_dir: self.wtf_dir,
            fonts_dir: self.fonts_dir,
        })
    }

    fn validate(&self) -> AppResult<()> {
        validate_absolute_installation_path("product root", &self.product_root)?;
        validate_absolute_installation_path("flavor root", &self.flavor_root)?;
        validate_absolute_installation_path("interface directory", &self.interface_dir)?;
        validate_absolute_installation_path("addon directory", &self.addon_dir)?;
        validate_absolute_installation_path("WTF directory", &self.wtf_dir)?;
        validate_absolute_installation_path("fonts directory", &self.fonts_dir)?;

        Ok(())
    }
}

fn validate_absolute_installation_path(description: &str, path: &Path) -> AppResult<()> {
    if path.is_absolute() {
        return Ok(());
    }

    Err(AppError::Validation(format!(
        "resolved installation {description} must be absolute: {}",
        path.display()
    )))
}
