use std::path::PathBuf;

use crate::core::app::{HostPlatformValue, ResolvedInstallationValue, WowFlavorValue};

pub(crate) fn sample_installation() -> ResolvedInstallationValue {
    ResolvedInstallationValue {
        platform: HostPlatformValue::Windows,
        flavor: WowFlavorValue::Retail,
        product_root: PathBuf::from("C:\\Games\\World of Warcraft"),
        flavor_root: PathBuf::from("C:\\Games\\World of Warcraft\\_retail_"),
        interface_dir: PathBuf::from("C:\\Games\\World of Warcraft\\_retail_\\Interface"),
        addon_dir: PathBuf::from("C:\\Games\\World of Warcraft\\_retail_\\Interface\\AddOns"),
        wtf_dir: PathBuf::from("C:\\Games\\World of Warcraft\\_retail_\\WTF"),
        fonts_dir: PathBuf::from("C:\\Games\\World of Warcraft\\_retail_\\Fonts"),
    }
}
