use serde::Serialize;

use crate::core::addon::{
    AddonProvider, AddonSearchCatalog as DomainAddonSearchCatalog,
    AddonSearchResult as DomainAddonSearchResult,
};

use super::super::super::map_owned_vec;
use super::source::AddonSourceResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchResult {
    pub provider: String,
    pub name: String,
    pub summary: Option<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub install_hint: String,
    pub website_url: Option<String>,
    pub provider_project_id: Option<u32>,
    pub provider_file_id: Option<u32>,
    pub download_count: u64,
}

impl AddonSearchResult {
    pub(crate) fn from_domain_with_provider<P>(value: DomainAddonSearchResult, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();

        Self {
            provider: value.provider.to_string(),
            name: value.name,
            summary: value.summary,
            source,
            source_label,
            install_hint: value.install_hint,
            website_url: value.website_url,
            provider_project_id: value.provider_project_id,
            provider_file_id: value.provider_file_id,
            download_count: value.download_count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchCatalogResult {
    pub query: String,
    pub result_count: usize,
    pub results: Vec<AddonSearchResult>,
}

impl AddonSearchCatalogResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonSearchCatalog,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let result_count = value.results.len();

        Self {
            query: value.query,
            result_count,
            results: map_owned_vec(value.results, |value| {
                AddonSearchResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}
