use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::addon::{
    AddonCacheRepairRemotePolicyValue, AddonStatePathsValue, AddonStateStorageValue,
    AppRuntimeCapabilitiesValue, HttpNoValidatorCachePolicyValue,
};
use super::install::{HostPlatformValue, ResolvedInstallationValue};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperStrategyValue {
    #[default]
    NativeRust,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalHelperPolicyValue {
    #[default]
    NativeOnly,
    PreferExternal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalHelperAvailabilityValue {
    NotRequested,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkProxyDiagnosticsValue {
    pub http_proxy: bool,
    pub https_proxy: bool,
    pub all_proxy: bool,
    pub no_proxy: bool,
}

impl NetworkProxyDiagnosticsValue {
    pub fn is_empty(&self) -> bool {
        !self.http_proxy && !self.https_proxy && !self.all_proxy && !self.no_proxy
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderCredentialDiagnosticsValue {
    pub github_token: bool,
    pub curseforge_api_key: bool,
}

impl ProviderCredentialDiagnosticsValue {
    pub fn is_empty(&self) -> bool {
        !self.github_token && !self.curseforge_api_key
    }
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
    pub relative_path_base: Option<PathBuf>,
    pub default_backup_dir: Option<PathBuf>,
    pub default_bundle_output_dir: Option<PathBuf>,
    pub network_proxy: NetworkProxyDiagnosticsValue,
    pub provider_credentials: ProviderCredentialDiagnosticsValue,
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
    #[serde(default)]
    pub addon_cache_repair_remote_policy: Option<AddonCacheRepairRemotePolicyValue>,
    #[serde(default)]
    pub addon_search_cache_ttl_secs: Option<u64>,
}

impl RuntimeSettingsValue {
    pub(crate) fn is_empty(&self) -> bool {
        self.addon_state_storage.is_none()
            && self.addon_cache_dir.is_none()
            && self.http_no_validator_cache_policy.is_none()
            && self.addon_cache_repair_remote_policy.is_none()
            && self.addon_search_cache_ttl_secs.is_none()
    }
}
