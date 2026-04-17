mod addon_lock;
mod addon_source_archive;
mod apply;
mod apply_model;
mod apply_policy;
mod archive_read;
mod character_mapping;
mod character_mapping_match;
mod entry_layout;
mod entry_plan;
mod execution;
mod external_package;
mod packing;
mod planner;
mod shared;
mod target_accounts;
#[cfg(test)]
mod tests;
mod types;
mod wtf_archive;
mod wtf_scope;
mod zip_write;

use std::fs;
use std::path::{Path, PathBuf};

use self::addon_lock::ExtractedAddonLock;
pub use self::addon_lock::{apply_bundle_addon_lock, plan_bundle_addon_lock};
use self::addon_source_archive::{
    add_bundle_addon_sources_to_zip, read_generated_addon_lock, resolve_addon_index_paths,
};
pub use self::apply::{unpack_bundle, unpack_bundle_task};
use self::apply_model::{
    PlannedCleanup, PlannedEntry, PreparedApplyOperation, PreparedApplySource, PreparedBundleApply,
};
use self::apply_policy::{
    apply_action_order, apply_group_order, build_cleanup_operations, cleanup_scope_for_entry,
    resource_policy_for_group,
};
use self::archive_read::{
    collect_bundle_entry_names, count_bundle_entries, extract_embedded_addon_lock,
    read_bundle_entry_bytes_from_archive, read_manifest_from_archive,
};
use self::character_mapping::build_character_mappings;
use self::entry_plan::plan_extractable_entries;
use self::execution::{
    execute_apply_operations, file_contents_equal_to_bytes, rollback_or_report_apply_error,
};
pub use self::external_package::{
    AnalyzeExternalPackageRequest, AppliedExternalPackage, ApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest, ExternalPackageAnalysis, ExternalPackageApplyPlan,
    ExternalPackageEntry, ExternalPackageSourceKind, ExternalPackageSummary,
    ExternalPackageWarning, ExternalPackageWarningCategory, ExternalPackageWarningCode,
    ExternalPackageWarningGroup, PlanExternalPackageApplyRequest, PreparedExternalPackageBundle,
    analyze_external_package, analyze_external_package_task, apply_external_package,
    apply_external_package_task, create_external_package_bundle, plan_external_package_apply,
    plan_external_package_apply_task,
};
pub use self::packing::{inspect_bundle, load_apply_mappings, pack_bundle};
pub use self::planner::plan_bundle_apply;
use self::shared::{
    BundleAddonSourceEntry, BundleAddonSourceIndex, join_segments, resolve_zip_style_path,
    safe_file_part, safe_zip_segments, should_skip_path, to_zip_path, validate_plain_name,
    zip_dir_options, zip_file_options,
};
use self::target_accounts::{resolve_selected_target_accounts, validate_target_compatibility};
pub use self::types::{
    ApplyAction, ApplyGroup, ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary,
    BundleAddonLockApply, BundleAddonLockApplyRequest, BundleAddonLockPlan, BundleApplyMappings,
    BundleApplyPlan, BundleEntryCounts, BundleInspection, CharacterMappingOverride, CreatedBundle,
    GroupPolicy, HelperStrategy, PackBundleRequest, UnpackBundleRequest, UnpackedBundle, WtfScope,
};
use self::wtf_archive::{
    add_character_wtf_to_zip, add_common_wtf_to_zip, resolve_character_account,
};
use self::zip_write::{add_path_to_zip, write_toml_to_zip};
use crate::core::addon::lock::{
    AddonLock, AddonLockPackage, AddonLockSourceOverride, addon_lock_package_comparison_key,
    write_addon_lock,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup, restore_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, LocalWowAccount, discover_local_accounts};
use crate::core::lua_patch::{CharacterMapping, LuaRewriteOptions, preview_lua_bytes_rewrite};
use crate::core::manifest::{
    BundleManifest, CharacterMappingMode, CharacterResource, ResourceApplyPolicy,
};

const MANIFEST_ENTRY: &str = "manifest.toml";
const ADDON_LOCK_ENTRY: &str = "metadata/addons/lock.toml";
const ADDON_INDEX_ENTRY_ROOT: &str = "metadata/addons/indexes";
const ADDON_SOURCE_INDEX_ENTRY: &str = "metadata/addons/sources.toml";
const ADDON_SOURCE_ENTRY_ROOT: &str = "metadata/addons/sources";
