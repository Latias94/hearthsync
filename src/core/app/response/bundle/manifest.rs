use serde::Serialize;

use crate::core::app::{
    BundleCharacterResourceValue, BundleManifestValue, BundleMappingRulesValue, BundlePackageValue,
    BundleSourceValue,
};
use crate::core::manifest::BundleResources;

use super::super::super::map_owned_vec;

pub type BundlePackageResult = BundlePackageValue;
pub type BundleSourceResult = BundleSourceValue;
pub type BundleCharacterResourceResult = BundleCharacterResourceValue;
pub type BundleMappingRulesResult = BundleMappingRulesValue;
pub type BundleManifestResult = BundleManifestValue;

#[derive(Debug, Clone, Serialize)]
pub struct BundleResourcesResult {
    pub addons: Vec<String>,
    pub addon_count: usize,
    pub wtf_common: bool,
    pub wtf_character_count: usize,
    pub wtf_characters: Vec<BundleCharacterResourceResult>,
    pub fonts: bool,
    pub interface_assets: Vec<String>,
    pub interface_asset_count: usize,
    pub addon_lock: bool,
    pub addon_indexes: Vec<String>,
}

impl BundleResourcesResult {
    pub(crate) fn from_domain(value: BundleResources) -> Self {
        let addon_count = value.addons.len();
        let wtf_character_count = value.wtf_characters.len();
        let interface_asset_count = value.interface_assets.len();

        Self {
            addons: value.addons,
            addon_count,
            wtf_common: value.wtf_common,
            wtf_character_count,
            wtf_characters: map_owned_vec(
                value.wtf_characters,
                BundleCharacterResourceResult::from_domain,
            ),
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            interface_asset_count,
            addon_lock: value.addon_lock,
            addon_indexes: value.addon_indexes,
        }
    }
}
