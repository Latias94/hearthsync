use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostPlatform {
    Windows,
    MacOs,
    Linux,
    Unknown,
}

impl HostPlatform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "macos" => Self::MacOs,
            "linux" => Self::Linux,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WowFlavor {
    Retail,
    Classic,
    ClassicEra,
    Ptr,
    Beta,
    Xptr,
}

impl WowFlavor {
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

    pub fn from_folder_name(value: &str) -> Option<Self> {
        match value {
            "_retail_" => Some(Self::Retail),
            "_classic_" => Some(Self::Classic),
            "_classic_era_" => Some(Self::ClassicEra),
            "_ptr_" => Some(Self::Ptr),
            "_beta_" => Some(Self::Beta),
            "_xptr_" => Some(Self::Xptr),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedFlavorInstallation {
    pub platform: HostPlatform,
    pub product_root: PathBuf,
    pub flavor_root: PathBuf,
    pub flavor: WowFlavor,
    pub interface_dir: PathBuf,
    pub addon_dir: PathBuf,
    pub wtf_dir: PathBuf,
    pub fonts_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowAccount {
    pub account_name: String,
    pub account_dir: PathBuf,
    pub saved_variables_dir: PathBuf,
    pub characters: Vec<LocalWowCharacter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowCharacter {
    pub server: String,
    pub character: String,
    pub character_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProductInstallInspection {
    pub requested_path: PathBuf,
    pub product_root: PathBuf,
    pub available_flavors: Vec<WowFlavor>,
    pub installation: DetectedFlavorInstallation,
    pub health: InstallationHealth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Broken,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallationHealth {
    pub status: HealthStatus,
    pub missing_paths: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

impl InstallationHealth {
    pub fn summary(&self) -> &'static str {
        match self.status {
            HealthStatus::Healthy => "healthy",
            HealthStatus::Warning => "warning",
            HealthStatus::Broken => "broken",
        }
    }

    pub fn to_report(&self) -> String {
        let mut lines = vec![format!("Status: {}", self.summary())];

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
