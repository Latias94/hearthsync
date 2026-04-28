use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::addon::{
    AddonStatePathsValue, AddonStateStorageValue, AppRuntimeCapabilitiesValue,
    HttpNoValidatorCachePolicyValue,
};
use super::install::{HostPlatformValue, ResolvedInstallationValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperStrategyValue {
    NativeRust,
}

impl Default for HelperStrategyValue {
    fn default() -> Self {
        Self::NativeRust
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalHelperPolicyValue {
    NativeOnly,
    PreferExternal,
}

impl Default for ExternalHelperPolicyValue {
    fn default() -> Self {
        Self::NativeOnly
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalHelperAvailabilityValue {
    NotRequested,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalHelperCapabilitiesValue {
    pub policy: ExternalHelperPolicyValue,
    pub availability: ExternalHelperAvailabilityValue,
    pub active_strategy: HelperStrategyValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRuntimeDiagnosticsValue {
    pub host_platform: HostPlatformValue,
    pub install_scan_roots: Option<Vec<PathBuf>>,
    pub default_backup_dir: Option<PathBuf>,
    pub default_bundle_output_dir: Option<PathBuf>,
    pub selected_installation: Option<ResolvedInstallationValue>,
    pub addon_state_paths: Option<AddonStatePathsValue>,
    pub capabilities: AppRuntimeCapabilitiesValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSettingsValue {
    #[serde(default)]
    pub addon_state_storage: Option<AddonStateStorageValue>,
    #[serde(default)]
    pub addon_cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub http_no_validator_cache_policy: Option<HttpNoValidatorCachePolicyValue>,
}

impl RuntimeSettingsValue {
    pub(crate) fn is_empty(&self) -> bool {
        self.addon_state_storage.is_none()
            && self.addon_cache_dir.is_none()
            && self.http_no_validator_cache_policy.is_none()
    }
}
