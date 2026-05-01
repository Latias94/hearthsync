use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::lock::AddonLockPackage;

use super::super::super::map_owned_vec;
use super::super::addon::{AddonSourceResult, TrackedAddonResult};

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageResult {
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_directories: Vec<String>,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
}

impl AddonLockPackageResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonLockPackage, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            package_id: value.package_id,
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            name: value.name,
            version: value.version,
            source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            content_sha256: value.content_sha256,
            installed_at: value.installed_at,
            updated_at: value.updated_at,
            addon_directories: value.addon_directories,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
        }
    }
}
