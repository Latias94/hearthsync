use serde::Serialize;

use crate::core::addon::{
    AddonProvider, AddonSearchCatalog as DomainAddonSearchCatalog,
    AddonSearchProviderFailure as DomainAddonSearchProviderFailure,
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
pub struct AddonSearchProviderFailureResult {
    pub provider_id: String,
    pub provider_name: String,
    pub source_family: String,
    pub message: String,
}

impl AddonSearchProviderFailureResult {
    pub(crate) fn from_domain(value: DomainAddonSearchProviderFailure) -> Self {
        Self {
            provider_id: value.provider_id,
            provider_name: value.provider_name,
            source_family: value.source_family,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchCatalogResult {
    pub query: String,
    pub provider_id: Option<String>,
    pub result_count: usize,
    pub failure_count: usize,
    pub results: Vec<AddonSearchResult>,
    pub failures: Vec<AddonSearchProviderFailureResult>,
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
        let failure_count = value.failures.len();

        Self {
            query: value.query,
            provider_id: value.provider_id,
            result_count,
            failure_count,
            results: map_owned_vec(value.results, |value| {
                AddonSearchResult::from_domain_with_provider(value, provider)
            }),
            failures: map_owned_vec(
                value.failures,
                AddonSearchProviderFailureResult::from_domain,
            ),
        }
    }
}
