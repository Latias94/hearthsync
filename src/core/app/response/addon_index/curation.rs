use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::index::{
    AddonIndexPackageSuggestion as DomainAddonIndexPackageSuggestion,
    AddonIndexPackageSuggestionStatus as DomainAddonIndexPackageSuggestionStatus,
    AddonIndexScaffoldResult as DomainAddonIndexScaffoldResult,
    AddonIndexSuggestion as DomainAddonIndexSuggestion,
};

use super::super::super::map_owned_vec;
use super::shared::AddonIndexTrackedMatchStrategyResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexSuggestionResult {
    pub index_path: PathBuf,
    pub index_name: String,
    pub index_package_count: usize,
    pub considered_package_count: usize,
    pub suggested_package_count: usize,
    pub complete_package_count: usize,
    pub no_match_package_count: usize,
    pub ambiguous_match_package_count: usize,
    pub skipped_unsupported_flavor_package_count: usize,
    pub packages: Vec<AddonIndexPackageSuggestionResult>,
}

impl AddonIndexSuggestionResult {
    pub(crate) fn from_domain(value: DomainAddonIndexSuggestion) -> Self {
        Self {
            index_path: value.index_path,
            index_name: value.index_name,
            index_package_count: value.index_package_count,
            considered_package_count: value.considered_package_count,
            suggested_package_count: value.suggested_package_count,
            complete_package_count: value.complete_package_count,
            no_match_package_count: value.no_match_package_count,
            ambiguous_match_package_count: value.ambiguous_match_package_count,
            skipped_unsupported_flavor_package_count: value
                .skipped_unsupported_flavor_package_count,
            packages: map_owned_vec(
                value.packages,
                AddonIndexPackageSuggestionResult::from_domain,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexScaffoldResult {
    pub index_path: PathBuf,
    pub index_name: String,
    pub package_count: usize,
    pub used_metadata_package_count: usize,
    pub inferred_name_package_count: usize,
    pub inferred_version_package_count: usize,
    pub placeholder_version_package_count: usize,
    pub package_ids: Vec<String>,
}

impl AddonIndexScaffoldResult {
    pub(crate) fn from_domain(value: DomainAddonIndexScaffoldResult) -> Self {
        Self {
            index_path: value.index_path,
            index_name: value.index_name,
            package_count: value.package_count,
            used_metadata_package_count: value.used_metadata_package_count,
            inferred_name_package_count: value.inferred_name_package_count,
            inferred_version_package_count: value.inferred_version_package_count,
            placeholder_version_package_count: value.placeholder_version_package_count,
            package_ids: value.package_ids,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexPackageSuggestionStatusResult {
    Suggested,
    Complete,
    NoLocalMatch,
    AmbiguousLocalMatch,
}

impl AddonIndexPackageSuggestionStatusResult {
    fn from_domain(value: DomainAddonIndexPackageSuggestionStatus) -> Self {
        match value {
            DomainAddonIndexPackageSuggestionStatus::Suggested => Self::Suggested,
            DomainAddonIndexPackageSuggestionStatus::Complete => Self::Complete,
            DomainAddonIndexPackageSuggestionStatus::NoLocalMatch => Self::NoLocalMatch,
            DomainAddonIndexPackageSuggestionStatus::AmbiguousLocalMatch => {
                Self::AmbiguousLocalMatch
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexPackageSuggestionResult {
    pub package_id: String,
    pub package_name: String,
    pub current_match_package_ids: Vec<String>,
    pub current_addon_directories: Vec<String>,
    pub status: AddonIndexPackageSuggestionStatusResult,
    pub matched_tracked_package_id: Option<String>,
    pub match_strategy: Option<AddonIndexTrackedMatchStrategyResult>,
    pub matched_addon_directories: Vec<String>,
    pub match_package_ids_to_add: Vec<String>,
    pub addon_directories_to_add: Vec<String>,
    pub message: String,
}

impl AddonIndexPackageSuggestionResult {
    fn from_domain(value: DomainAddonIndexPackageSuggestion) -> Self {
        Self {
            package_id: value.package_id,
            package_name: value.package_name,
            current_match_package_ids: value.current_match_package_ids,
            current_addon_directories: value.current_addon_directories,
            status: AddonIndexPackageSuggestionStatusResult::from_domain(value.status),
            matched_tracked_package_id: value.matched_tracked_package_id,
            match_strategy: value
                .match_strategy
                .map(AddonIndexTrackedMatchStrategyResult::from_domain),
            matched_addon_directories: value.matched_addon_directories,
            match_package_ids_to_add: value.match_package_ids_to_add,
            addon_directories_to_add: value.addon_directories_to_add,
            message: value.message,
        }
    }
}
