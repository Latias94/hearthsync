use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::index::AddonIndexSearch as DomainAddonIndexSearch;

use super::super::super::map_owned_vec;
use super::package::AddonIndexPackageResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexSearchResult {
    pub index_path: PathBuf,
    pub index_name: String,
    pub query: String,
    pub package_count: usize,
    pub matched_package_count: usize,
    pub returned_package_count: usize,
    pub packages: Vec<AddonIndexPackageResult>,
}

impl AddonIndexSearchResult {
    pub(crate) fn from_domain_with_provider<P>(value: DomainAddonIndexSearch, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            index_path: value.index_path,
            index_name: value.index_name,
            query: value.query,
            package_count: value.package_count,
            matched_package_count: value.matched_package_count,
            returned_package_count: value.returned_package_count,
            packages: map_owned_vec(value.packages, |value| {
                AddonIndexPackageResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}
