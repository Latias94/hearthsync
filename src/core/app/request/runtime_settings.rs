use std::path::PathBuf;

use crate::core::app::{
    AddonCacheRepairRemotePolicyValue, AddonStateStorageValue, HttpNoValidatorCachePolicyValue,
};

#[derive(Debug, Clone)]
pub struct SetRuntimeSettingsAppRequest {
    pub addon_state_storage: Option<AddonStateStorageValue>,
    pub clear_addon_state_storage: bool,
    pub addon_cache_dir: Option<PathBuf>,
    pub clear_addon_cache_dir: bool,
    pub http_no_validator_cache_policy: Option<HttpNoValidatorCachePolicyValue>,
    pub clear_http_no_validator_cache_policy: bool,
    pub addon_cache_repair_remote_policy: Option<AddonCacheRepairRemotePolicyValue>,
    pub clear_addon_cache_repair_remote_policy: bool,
    pub addon_search_cache_ttl_secs: Option<u64>,
    pub clear_addon_search_cache_ttl_secs: bool,
}
