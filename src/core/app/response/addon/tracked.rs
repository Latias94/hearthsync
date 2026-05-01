use serde::Serialize;

use crate::core::addon::{AddonProvider, TrackedAddon, TrackedAddonPackage};
use crate::core::app::AddonPackageMetadataValue;

use super::super::super::map_owned_vec;
use super::source::AddonSourceResult;

#[derive(Debug, Clone, Serialize)]
pub struct TrackedAddonResult {
    pub directory_name: String,
    pub toc_file: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
}

impl TrackedAddonResult {
    pub(crate) fn from_domain(value: TrackedAddon) -> Self {
        Self {
            directory_name: value.directory_name,
            toc_file: value.toc_file,
            title: value.title,
            version: value.version,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackedAddonPackageResult {
    pub package_id: String,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub metadata: Option<AddonPackageMetadataValue>,
}

impl TrackedAddonPackageResult {
    pub(crate) fn from_domain_with_provider<P>(value: TrackedAddonPackage, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            source,
            source_label,
            installed_at: value.installed_at,
            updated_at: value.updated_at,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
            metadata: value.metadata.map(AddonPackageMetadataValue::from_domain),
        }
    }
}
