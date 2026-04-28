use std::path::PathBuf;

use crate::core::app::{AddonStateStorageValue, HttpNoValidatorCachePolicyValue};

#[derive(Debug, Clone)]
pub struct SetRuntimeSettingsAppRequest {
    pub addon_state_storage: Option<AddonStateStorageValue>,
    pub clear_addon_state_storage: bool,
    pub addon_cache_dir: Option<PathBuf>,
    pub clear_addon_cache_dir: bool,
    pub http_no_validator_cache_policy: Option<HttpNoValidatorCachePolicyValue>,
    pub clear_http_no_validator_cache_policy: bool,
}
