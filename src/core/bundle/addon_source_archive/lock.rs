use std::path::Path;

use super::super::*;

pub(in crate::core::bundle) fn read_generated_addon_lock(path: &Path) -> AppResult<AddonLock> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}
