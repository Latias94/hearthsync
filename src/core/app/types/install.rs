use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
        DomainHostPlatform::current().into()
    }
}

impl From<DomainHostPlatform> for HostPlatformValue {
    fn from(value: DomainHostPlatform) -> Self {
        match value {
            DomainHostPlatform::Windows => Self::Windows,
            DomainHostPlatform::MacOs => Self::MacOs,
            DomainHostPlatform::Linux => Self::Linux,
            DomainHostPlatform::Unknown => Self::Unknown,
        }
    }
}

impl From<HostPlatformValue> for DomainHostPlatform {
    fn from(value: HostPlatformValue) -> Self {
        match value {
            HostPlatformValue::Windows => Self::Windows,
            HostPlatformValue::MacOs => Self::MacOs,
            HostPlatformValue::Linux => Self::Linux,
            HostPlatformValue::Unknown => Self::Unknown,
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
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retail => "retail",
            Self::Classic => "classic",
            Self::ClassicEra => "classic_era",
            Self::Ptr => "ptr",
            Self::Beta => "beta",
            Self::Xptr => "xptr",
        }
    }

    pub fn folder_name(&self) -> &'static str {
        match self {
            Self::Retail => "_retail_",
            Self::Classic => "_classic_",
            Self::ClassicEra => "_classic_era_",
            Self::Ptr => "_ptr_",
            Self::Beta => "_beta_",
            Self::Xptr => "_xptr_",
        }
    }
}

impl From<DomainWowFlavor> for WowFlavorValue {
    fn from(value: DomainWowFlavor) -> Self {
        match value {
            DomainWowFlavor::Retail => Self::Retail,
            DomainWowFlavor::Classic => Self::Classic,
            DomainWowFlavor::ClassicEra => Self::ClassicEra,
            DomainWowFlavor::Ptr => Self::Ptr,
            DomainWowFlavor::Beta => Self::Beta,
            DomainWowFlavor::Xptr => Self::Xptr,
        }
    }
}

impl From<WowFlavorValue> for DomainWowFlavor {
    fn from(value: WowFlavorValue) -> Self {
        match value {
            WowFlavorValue::Retail => Self::Retail,
            WowFlavorValue::Classic => Self::Classic,
            WowFlavorValue::ClassicEra => Self::ClassicEra,
            WowFlavorValue::Ptr => Self::Ptr,
            WowFlavorValue::Beta => Self::Beta,
            WowFlavorValue::Xptr => Self::Xptr,
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

impl From<DomainHealthStatus> for HealthStatusValue {
    fn from(value: DomainHealthStatus) -> Self {
        match value {
            DomainHealthStatus::Healthy => Self::Healthy,
            DomainHealthStatus::Warning => Self::Warning,
            DomainHealthStatus::Broken => Self::Broken,
        }
    }
}

impl From<HealthStatusValue> for DomainHealthStatus {
    fn from(value: HealthStatusValue) -> Self {
        match value {
            HealthStatusValue::Healthy => Self::Healthy,
            HealthStatusValue::Warning => Self::Warning,
            HealthStatusValue::Broken => Self::Broken,
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

impl From<DetectedFlavorInstallation> for ResolvedInstallationValue {
    fn from(value: DetectedFlavorInstallation) -> Self {
        Self {
            platform: value.platform.into(),
            flavor: value.flavor.into(),
            product_root: value.product_root,
            flavor_root: value.flavor_root,
            interface_dir: value.interface_dir,
            addon_dir: value.addon_dir,
            wtf_dir: value.wtf_dir,
            fonts_dir: value.fonts_dir,
        }
    }
}

impl From<ResolvedInstallationValue> for DetectedFlavorInstallation {
    fn from(value: ResolvedInstallationValue) -> Self {
        Self {
            platform: value.platform.into(),
            flavor: value.flavor.into(),
            product_root: value.product_root,
            flavor_root: value.flavor_root,
            interface_dir: value.interface_dir,
            addon_dir: value.addon_dir,
            wtf_dir: value.wtf_dir,
            fonts_dir: value.fonts_dir,
        }
    }
}
