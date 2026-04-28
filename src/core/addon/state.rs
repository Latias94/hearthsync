use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::error::AppResult;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform};
use crate::core::platform_dirs::app_data_subdir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonStateStorageKind {
    AppData,
    Sidecar,
}

impl Default for AddonStateStorageKind {
    fn default() -> Self {
        Self::AppData
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AddonStatePaths {
    pub(crate) root_dir: PathBuf,
    pub(crate) registry_path: PathBuf,
    pub(crate) lock_path: PathBuf,
    pub(crate) policy_path: PathBuf,
    pub(crate) adopted_dir: PathBuf,
}

impl AddonStatePaths {
    pub fn for_installation(
        storage: AddonStateStorageKind,
        installation: &DetectedFlavorInstallation,
    ) -> AppResult<Self> {
        let root_dir = match storage {
            AddonStateStorageKind::AppData => app_data_root(installation)?,
            AddonStateStorageKind::Sidecar => installation.addon_dir.join(".hearthsync"),
        };

        Ok(Self {
            registry_path: root_dir.join("addons.toml"),
            lock_path: root_dir.join("lock.toml"),
            policy_path: root_dir.join("addon-policy.toml"),
            adopted_dir: root_dir.join("adopted"),
            root_dir,
        })
    }
}

fn app_data_root(installation: &DetectedFlavorInstallation) -> AppResult<PathBuf> {
    app_data_subdir(
        Path::new("wow")
            .join(install_key(installation))
            .join(installation.flavor.as_str())
            .join("addons")
            .as_path(),
    )
}

fn install_key(installation: &DetectedFlavorInstallation) -> String {
    let display_name = installation
        .product_root
        .file_name()
        .and_then(|value| value.to_str())
        .map(slugify_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "installation".to_string());
    let digest = short_path_hash(&normalized_install_path(
        &installation.product_root,
        installation.platform,
    ));

    format!("{display_name}-{digest}")
}

fn normalized_install_path(path: &Path, platform: HostPlatform) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let trimmed = normalized.trim_end_matches('/');
    if matches!(platform, HostPlatform::Windows) {
        trimmed.to_ascii_lowercase()
    } else {
        trimmed.to_string()
    }
}

fn short_path_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut short = String::with_capacity(12);
    for byte in digest.iter().take(6) {
        short.push_str(&format!("{byte:02x}"));
    }
    short
}

fn slugify_component(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_was_separator = false;

    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
            continue;
        }

        if !previous_was_separator && !slug.is_empty() {
            slug.push('-');
            previous_was_separator = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    slug
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{AddonStatePaths, AddonStateStorageKind};
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

    #[test]
    fn sidecar_state_paths_live_under_addon_dir() {
        let installation = fixture_installation();
        let paths =
            AddonStatePaths::for_installation(AddonStateStorageKind::Sidecar, &installation)
                .expect("state paths");

        assert_eq!(paths.root_dir, installation.addon_dir.join(".hearthsync"));
        assert_eq!(paths.registry_path, paths.root_dir.join("addons.toml"));
        assert_eq!(paths.lock_path, paths.root_dir.join("lock.toml"));
        assert_eq!(paths.policy_path, paths.root_dir.join("addon-policy.toml"));
        assert_eq!(paths.adopted_dir, paths.root_dir.join("adopted"));
    }

    #[test]
    fn appdata_state_paths_use_stable_install_key_and_flavor_segment() {
        let installation = fixture_installation();
        let paths =
            AddonStatePaths::for_installation(AddonStateStorageKind::AppData, &installation)
                .expect("state paths");

        let path = paths.root_dir.to_string_lossy().replace('\\', "/");
        assert!(path.contains("/wow/"));
        assert!(path.contains("/retail/addons"));
        assert!(path.contains("world-of-warcraft-"));
        assert!(!path.contains("/hearthsync/hearthsync/"));
        assert!(!path.contains("/Interface/AddOns/.hearthsync"));
    }

    fn fixture_installation() -> DetectedFlavorInstallation {
        let product_root = PathBuf::from(r"E:\Games\World of Warcraft");
        let flavor_root = product_root.join("_retail_");

        DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root: product_root.clone(),
            flavor_root: flavor_root.clone(),
            flavor: WowFlavor::Retail,
            interface_dir: flavor_root.join("Interface"),
            addon_dir: flavor_root.join("Interface").join("AddOns"),
            wtf_dir: flavor_root.join("WTF"),
            fonts_dir: flavor_root.join("Fonts"),
        }
    }
}
