use crate::core::app::{
    AddonCachePurgeResult, AddonCacheRepairRemotePolicyValue, AddonCacheRepairResult,
    AddonIndexAttachPackageStatusResult, AddonIndexAttachResult, AddonIndexInspectionResult,
    AddonIndexInspectionWarningCodeResult, AddonIndexInspectionWarningSeverityResult,
    AddonIndexInstallResult, AddonIndexPackageSuggestionStatusResult, AddonIndexRelinkResult,
    AddonIndexScaffoldResult, AddonIndexSearchResult, AddonIndexSuggestionResult,
    AddonIndexTrackedMatchStrategyResult, AddonIndexUpdateResult, AddonIndexValidationResult,
    AddonInventoryResult, AddonSearchCatalogResult, AdoptedAddonPackageResult,
    InstalledAddonPackageResult, RelinkedAddonPackageResult, RemovedAddonPackageResult,
    TrackedAddonPackageResult, TrackedAddonResult, UpdatedAddonPackageResult,
};

use super::shared::{format_optional_path_or_none, format_string_list_or_none};

mod cache;
mod index;
mod manage;
mod shared;
#[cfg(test)]
mod tests;

pub(in crate::cli) use self::cache::{render_addon_cache_purge, render_addon_cache_repair};
pub(in crate::cli) use self::index::{
    render_addon_index_attach, render_addon_index_inspection, render_addon_index_install,
    render_addon_index_relink, render_addon_index_scaffold, render_addon_index_search,
    render_addon_index_suggestion, render_addon_index_update, render_addon_index_validation,
};
pub(in crate::cli) use self::manage::{
    render_addon_adopt, render_addon_install, render_addon_inventory, render_addon_relink,
    render_addon_remove, render_addon_search_catalog, render_addon_update,
};
use self::shared::{
    format_addon_index_attach_status, format_addon_index_match_strategy,
    format_addon_index_suggestion_status, format_addon_index_warning_code,
    format_addon_index_warning_severity, format_tracked_addon_names,
    format_tracked_package_summaries,
};
