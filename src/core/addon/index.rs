mod curation;
mod matching;
mod operations;
mod storage;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::addon::AddonStatePaths;
use crate::core::addon::{AddonSourceRef, InstalledAddonPackageResult, UpdatedAddonPackageResult};
use crate::core::install::DetectedFlavorInstallation;

pub use self::curation::{scaffold_addon_index, suggest_addon_index_hints};
pub use self::operations::{
    attach_addons_from_index, attach_addons_from_index_task, install_addon_from_index,
    install_addon_from_index_task, relink_addon_from_index, relink_addon_from_index_task,
    update_addons_from_index, update_addons_from_index_task,
};
pub(crate) use self::operations::{
    attach_addons_from_index_task_with_provider, install_addon_from_index_task_with_provider,
    relink_addon_from_index_task_with_provider, update_addons_from_index_task_with_provider,
    validate_addon_index_update_dependency_policy_support,
};
pub use self::storage::inspect_addon_index;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonIndex {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub packages: Vec<AddonIndexPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonIndexPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub match_package_ids: Vec<String>,
    pub source: AddonSourceRef,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub addon_directories: Vec<String>,
    #[serde(default)]
    pub supported_flavors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspection {
    pub index_path: PathBuf,
    pub index: AddonIndex,
    pub package_count: usize,
    pub identity_hint_coverage: AddonIndexIdentityHintCoverage,
    pub warning_count: usize,
    pub blocking_warning_count: usize,
    pub advisory_warning_count: usize,
    pub warnings: Vec<AddonIndexInspectionWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexIdentityHintCoverage {
    pub package_count_with_both_exact_hints: usize,
    pub package_count_with_any_exact_hints: usize,
    pub package_count_with_match_package_ids: usize,
    pub package_count_with_addon_directories: usize,
    pub package_count_without_match_package_ids: usize,
    pub package_count_without_addon_directories: usize,
    pub package_count_without_exact_hints: usize,
    pub packages_without_match_package_ids: Vec<String>,
    pub packages_without_addon_directories: Vec<String>,
    pub packages_without_exact_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexInspectionWarningCode {
    MissingMatchPackageIds,
    MissingAddonDirectories,
    MissingExactIdentityHints,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexInspectionWarningSeverity {
    Blocking,
    Advisory,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspectionWarning {
    pub code: AddonIndexInspectionWarningCode,
    pub severity: AddonIndexInspectionWarningSeverity,
    pub package_id: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AddonIndexInstallRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub index_path: PathBuf,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInstallResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackage,
    pub install: InstalledAddonPackageResult,
}

#[derive(Debug, Clone)]
pub struct AddonIndexAttachRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub apply_ready_only: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexAttachResult {
    pub index_path: PathBuf,
    pub index_name: String,
    pub dry_run: bool,
    pub ready: bool,
    pub applied: bool,
    pub partial_apply: bool,
    pub registry_path: PathBuf,
    pub index_package_count: usize,
    pub considered_package_count: usize,
    pub change_package_count: usize,
    pub attached_package_count: usize,
    pub already_attached_package_count: usize,
    pub blocked_package_count: usize,
    pub skipped_unsupported_flavor_package_count: usize,
    pub packages: Vec<AddonIndexAttachPackageResult>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexAttachPackageStatus {
    WouldAttach,
    Attached,
    AlreadyAttached,
    NoLocalMatch,
    AmbiguousLocalMatch,
    AddonDirectoryMismatch,
    PrepareFailed,
    SkippedUnsupportedFlavor,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexAttachPackageResult {
    pub package: AddonIndexPackage,
    pub status: AddonIndexAttachPackageStatus,
    pub matched_tracked_package_id: Option<String>,
    pub match_strategy: Option<AddonIndexTrackedMatchStrategy>,
    pub previous_source: Option<AddonSourceRef>,
    pub source: Option<AddonSourceRef>,
    pub source_changed: bool,
    pub metadata_changed: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AddonIndexUpdateRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexUpdateResult {
    pub index_path: PathBuf,
    pub selected_packages: Vec<AddonIndexPackage>,
    pub update: UpdatedAddonPackageResult,
}

#[derive(Debug, Clone)]
pub struct AddonIndexRelinkRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub index_path: PathBuf,
    pub name: String,
    pub target: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexRelinkResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackage,
    pub dry_run: bool,
    pub tracked_package_id: String,
    pub previous_source: AddonSourceRef,
    pub source: AddonSourceRef,
    pub addons: Vec<crate::core::addon::TrackedAddon>,
    pub metadata: crate::core::addon::AddonPackageMetadata,
    pub registry_path: PathBuf,
    pub source_changed: bool,
    pub metadata_changed: bool,
}

#[derive(Debug, Clone)]
pub struct AddonIndexSuggestionRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub index_path: PathBuf,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexSuggestion {
    pub index_path: PathBuf,
    pub index_name: String,
    pub index_package_count: usize,
    pub considered_package_count: usize,
    pub suggested_package_count: usize,
    pub complete_package_count: usize,
    pub no_match_package_count: usize,
    pub ambiguous_match_package_count: usize,
    pub skipped_unsupported_flavor_package_count: usize,
    pub packages: Vec<AddonIndexPackageSuggestion>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexPackageSuggestionStatus {
    Suggested,
    Complete,
    NoLocalMatch,
    AmbiguousLocalMatch,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexTrackedMatchStrategy {
    StoredIndexPackageId,
    ExactPackageId,
    CuratedMatchPackageId,
    SourceIdentity,
    SourceFamilyIdentity,
    DisplayName,
    AddonDirectories,
    AddonDirectoryOverlap,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexPackageSuggestion {
    pub package_id: String,
    pub package_name: String,
    pub current_match_package_ids: Vec<String>,
    pub current_addon_directories: Vec<String>,
    pub status: AddonIndexPackageSuggestionStatus,
    pub matched_tracked_package_id: Option<String>,
    pub match_strategy: Option<AddonIndexTrackedMatchStrategy>,
    pub matched_addon_directories: Vec<String>,
    pub match_package_ids_to_add: Vec<String>,
    pub addon_directories_to_add: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AddonIndexScaffoldRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub index_path: PathBuf,
    pub index_name: String,
    pub description: Option<String>,
    pub name: Option<String>,
    pub overwrite: bool,
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
