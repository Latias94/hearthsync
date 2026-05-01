use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::index::AddonIndexPackage;

use super::super::addon::AddonSourceResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexPackageResult {
    pub id: String,
    pub name: String,
    pub version: String,
    pub match_package_ids: Vec<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub sha256: Option<String>,
    pub addon_directories: Vec<String>,
    pub supported_flavors: Vec<String>,
}

impl AddonIndexPackageResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonIndexPackage, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();

        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            match_package_ids: value.match_package_ids,
            source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            sha256: value.sha256,
            addon_directories: value.addon_directories,
            supported_flavors: value.supported_flavors,
        }
    }
}
