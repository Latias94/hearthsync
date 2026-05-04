use std::path::PathBuf;

use super::super::test_support::{
    sample_index_package, sample_source, sample_tracked_addon, sample_tracked_package,
};
use super::{
    render_addon_adopt, render_addon_cache_purge, render_addon_cache_repair,
    render_addon_index_attach, render_addon_index_inspection, render_addon_index_install,
    render_addon_index_relink, render_addon_index_scaffold, render_addon_index_suggestion,
    render_addon_index_update, render_addon_index_validation, render_addon_install,
    render_addon_inventory, render_addon_relink, render_addon_remove, render_addon_search_catalog,
    render_addon_update,
};
use crate::core::app::{
    AddonCachePurgeResult, AddonCacheRepairResult, AddonIndexAttachPackageResult,
    AddonIndexAttachPackageStatusResult, AddonIndexAttachResult,
    AddonIndexIdentityHintCoverageResult, AddonIndexInspectionResult,
    AddonIndexInspectionWarningCodeResult, AddonIndexInspectionWarningResult,
    AddonIndexInspectionWarningSeverityResult, AddonIndexInstallResult,
    AddonIndexPackageSuggestionResult, AddonIndexPackageSuggestionStatusResult,
    AddonIndexRelinkResult, AddonIndexScaffoldResult, AddonIndexSuggestionResult,
    AddonIndexTrackedMatchStrategyResult, AddonIndexUpdateResult, AddonIndexValidationResult,
    AddonInventoryResult, AddonSearchCatalogResult, AddonSearchProviderFailureResult,
    AddonSearchResult, AdoptedAddonPackageResult, InstalledAddonPackageResult,
    RelinkedAddonPackageResult, RemovedAddonPackageResult, UpdatedAddonPackageResult,
};

mod cache;
mod index;
mod manage;
