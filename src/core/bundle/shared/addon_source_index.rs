use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::core::bundle) struct BundleAddonSourceIndex {
    pub(in crate::core::bundle) schema_version: u32,
    pub(in crate::core::bundle) sources: Vec<BundleAddonSourceEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::core::bundle) struct BundleAddonSourceEntry {
    pub(in crate::core::bundle) comparison_key: String,
    pub(in crate::core::bundle) package_id: String,
    pub(in crate::core::bundle) path: String,
    pub(in crate::core::bundle) content_sha256: String,
    pub(in crate::core::bundle) addon_directories: Vec<String>,
}
