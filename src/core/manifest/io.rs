use std::path::Path;

use crate::core::error::AppResult;

use super::BundleManifest;

pub fn load_manifest(path: &Path) -> AppResult<BundleManifest> {
    let content = std::fs::read_to_string(path)?;
    let manifest: BundleManifest = toml::from_str(&content)?;
    Ok(manifest)
}
