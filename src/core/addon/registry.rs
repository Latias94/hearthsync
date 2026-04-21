use std::fs;
use std::path::{Path, PathBuf};

use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

use super::{AddonRegistry, lock};

pub(crate) fn load_registry(installation: &DetectedFlavorInstallation) -> AppResult<AddonRegistry> {
    let path = registry_path(installation);
    if !path.exists() {
        return Ok(AddonRegistry::default());
    }

    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub(crate) fn save_registry(
    installation: &DetectedFlavorInstallation,
    registry: &AddonRegistry,
) -> AppResult<()> {
    let path = registry_path(installation);
    if registry.packages.is_empty() {
        cleanup_registry_storage(&path)?;
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_bytes_atomically(&path, toml::to_string_pretty(registry)?.as_bytes())?;
    lock::sync_addon_lock_from_registry(installation, registry)?;
    Ok(())
}

fn cleanup_registry_storage(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    let lock_path = path.with_file_name("lock.toml");
    if lock_path.exists() {
        fs::remove_file(lock_path)?;
    }

    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if !parent.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(parent)?;
    if entries.next().is_none() {
        fs::remove_dir(parent)?;
    }

    Ok(())
}

pub(super) fn registry_path(installation: &DetectedFlavorInstallation) -> PathBuf {
    installation
        .addon_dir
        .join(".hearthsync")
        .join("addons.toml")
}
