use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::core::error::{AppError, AppResult};

use super::model::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

pub(super) fn candidate_product_roots() -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();

    if let Ok(custom_root) = std::env::var("WOW_INSTALL_ROOT") {
        roots.insert(PathBuf::from(custom_root));
    }

    if let Some(base_dirs) = BaseDirs::new() {
        roots.extend(common_roots_for_platform(base_dirs.home_dir()));
    }

    roots.into_iter().collect()
}

pub(super) fn build_installation(
    product_root: &Path,
    flavor_root: &Path,
    flavor: WowFlavor,
) -> DetectedFlavorInstallation {
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    DetectedFlavorInstallation {
        platform: HostPlatform::current(),
        product_root: product_root.to_path_buf(),
        flavor_root: flavor_root.to_path_buf(),
        flavor,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

pub(super) fn detect_flavors(product_root: &Path) -> Vec<WowFlavor> {
    let mut flavors = Vec::new();

    for flavor in [
        WowFlavor::Retail,
        WowFlavor::Classic,
        WowFlavor::ClassicEra,
        WowFlavor::Ptr,
        WowFlavor::Beta,
        WowFlavor::Xptr,
    ] {
        let candidate = product_root.join(flavor.folder_name());
        if has_wow_structure(&candidate) {
            flavors.push(flavor);
        }
    }

    flavors
}

pub(super) fn has_wow_structure(path: &Path) -> bool {
    path.join("Interface").is_dir() && path.join("WTF").is_dir()
}

pub(super) fn normalize_path(path: &Path) -> AppResult<PathBuf> {
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    Ok(strip_windows_verbatim_prefix(
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
    ))
}

fn common_roots_for_platform(home_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = BTreeSet::new();

    match HostPlatform::current() {
        HostPlatform::Windows => {
            for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
                if let Ok(value) = std::env::var(variable) {
                    candidates.insert(PathBuf::from(value).join("World of Warcraft"));
                }
            }

            candidates.insert(home_dir.join("Games").join("World of Warcraft"));

            for drive in 'C'..='Z' {
                let root = PathBuf::from(format!("{drive}:\\"));
                candidates.insert(root.join("World of Warcraft"));
                candidates.insert(root.join("Games").join("World of Warcraft"));
                candidates.insert(root.join("Blizzard").join("World of Warcraft"));
                candidates.insert(root.join("Program Files").join("World of Warcraft"));
                candidates.insert(root.join("Program Files (x86)").join("World of Warcraft"));
            }
        }
        HostPlatform::MacOs => {
            candidates.insert(PathBuf::from("/Applications/World of Warcraft"));
            candidates.insert(home_dir.join("Applications").join("World of Warcraft"));
            candidates.insert(home_dir.join("Games").join("World of Warcraft"));
        }
        HostPlatform::Linux | HostPlatform::Unknown => {
            candidates.insert(home_dir.join("Games").join("World of Warcraft"));
        }
    }

    candidates.into_iter().collect()
}

fn strip_windows_verbatim_prefix(path: PathBuf) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(stripped) = raw.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}
