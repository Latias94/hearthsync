mod addon_lock;
mod apply;
mod apply_policy;
mod archive_io;
mod entry_plan;
mod execution;
mod packing;
mod planner;
mod shared;
mod target_resolution;
#[cfg(test)]
mod tests;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::addon_lock::ExtractedAddonLock;
pub use self::addon_lock::{apply_bundle_addon_lock, plan_bundle_addon_lock};
pub use self::apply::unpack_bundle;
use self::apply_policy::{
    apply_action_order, apply_group_order, build_cleanup_operations, cleanup_scope_for_entry,
    resource_policy_for_group,
};
use self::archive_io::{
    add_bundle_addon_sources_to_zip, add_character_wtf_to_zip, add_common_wtf_to_zip,
    add_path_to_zip, collect_bundle_entry_names, count_bundle_entries, extract_embedded_addon_lock,
    read_bundle_entry_bytes_from_archive, read_generated_addon_lock, read_manifest_from_archive,
    resolve_addon_index_paths, resolve_character_account, write_toml_to_zip,
};
use self::entry_plan::plan_extractable_entries;
use self::execution::{
    execute_apply_operations, file_contents_equal_to_bytes, rollback_or_report_apply_error,
};
pub use self::packing::{inspect_bundle, load_apply_mappings, pack_bundle};
pub use self::planner::plan_bundle_apply;
use self::shared::{
    BundleAddonSourceEntry, BundleAddonSourceIndex, join_segments, safe_file_part,
    safe_zip_segments, should_skip_path, to_zip_path, validate_plain_name, zip_dir_options,
    zip_file_options,
};
use self::target_resolution::{
    build_character_mappings, resolve_selected_target_accounts, validate_target_compatibility,
};
use crate::core::addon::lock::{
    AddonLock, AddonLockApplyResult, AddonLockPackage, AddonLockPlanResult,
    AddonLockSourceOverride, addon_lock_package_comparison_key, write_addon_lock,
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

#[derive(Debug, Clone)]
pub struct PackBundleRequest {
    pub installation: DetectedFlavorInstallation,
    pub manifest: BundleManifest,
    pub output_path: Option<PathBuf>,
    pub manifest_base_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBundle {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockPlan {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub plan: AddonLockPlanResult,
}

#[derive(Debug, Clone)]
pub struct BundleAddonLockApplyRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockApply {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub apply: AddonLockApplyResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleInspection {
    pub archive_path: PathBuf,
    pub manifest: BundleManifest,
    pub entries: BundleEntryCounts,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct BundleEntryCounts {
    pub total_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub metadata: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleApplyPlan {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<LocalWowAccount>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<CharacterMapping>,
    pub operations: Vec<ApplyOperation>,
    pub summary: ApplyPlanSummary,
    pub helper_strategy: HelperStrategy,
    pub group_policies: ApplyGroupPolicies,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOperation {
    pub group: ApplyGroup,
    pub wtf_scope: Option<WtfScope>,
    pub action: ApplyAction,
    pub archive_name: String,
    pub destination: PathBuf,
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
    pub rewrite_count: usize,
    pub rewrite_applied: bool,
}

#[derive(Debug, Clone)]
struct PreparedApplyOperation {
    group: ApplyGroup,
    wtf_scope: Option<WtfScope>,
    action: ApplyAction,
    archive_name: String,
    destination: PathBuf,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
    rewrite_applied: bool,
    rewrites: Vec<CharacterMapping>,
}

impl PreparedApplyOperation {
    fn from_cleanup(cleanup: PlannedCleanup) -> Self {
        Self {
            group: cleanup.group,
            wtf_scope: None,
            action: ApplyAction::Remove,
            archive_name: format!("[cleanup] {}", cleanup.destination.display()),
            destination: cleanup.destination,
            target_account: cleanup.target_account,
            target_server: cleanup.target_server,
            target_character: cleanup.target_character,
            rewrite_applied: false,
            rewrites: Vec::new(),
        }
    }

    fn from_entry(entry: &PlannedEntry, action: ApplyAction, rewrite_applied: bool) -> Self {
        Self {
            group: entry.group,
            wtf_scope: entry.wtf_scope,
            action,
            archive_name: entry.archive_name.clone(),
            destination: entry.destination.clone(),
            target_account: entry.target_account.clone(),
            target_server: entry.target_server.clone(),
            target_character: entry.target_character.clone(),
            rewrite_applied,
            rewrites: entry.rewrites.clone(),
        }
    }

    fn preview(&self) -> ApplyOperation {
        ApplyOperation {
            group: self.group,
            wtf_scope: self.wtf_scope,
            action: self.action,
            archive_name: self.archive_name.clone(),
            destination: self.destination.clone(),
            target_account: self.target_account.clone(),
            target_server: self.target_server.clone(),
            target_character: self.target_character.clone(),
            rewrite_count: self.rewrites.len(),
            rewrite_applied: self.rewrite_applied,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ApplyPlanSummary {
    pub files_to_add: usize,
    pub files_to_replace: usize,
    pub files_to_skip: usize,
    pub paths_to_remove: usize,
    pub files_to_preserve: usize,
    pub files_to_rewrite: usize,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyAction {
    Remove,
    Add,
    Replace,
    Skip,
    Preserve,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyGroup {
    Addons,
    WtfCommon,
    WtfCharacters,
    Fonts,
    InterfaceAssets,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WtfScope {
    GlobalConfig,
    AccountRootFile,
    AccountSavedVariables,
    CharacterSavedVariables,
    CharacterState,
    CacheLike,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperStrategy {
    NativeRust,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyGroupPolicies {
    pub addons: GroupPolicy,
    pub wtf_common: GroupPolicy,
    pub wtf_characters: GroupPolicy,
    pub fonts: GroupPolicy,
    pub interface_assets: GroupPolicy,
    pub metadata: GroupPolicy,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupPolicy {
    pub policy: ResourceApplyPolicy,
}

#[derive(Debug, Clone)]
pub struct UnpackBundleRequest {
    pub bundle_path: PathBuf,
    pub installation: DetectedFlavorInstallation,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnpackedBundle {
    pub bundle_path: PathBuf,
    pub target_flavor_root: PathBuf,
    pub dry_run: bool,
    pub planned_files: usize,
    pub written_files: usize,
    pub rewritten_files: usize,
    pub backup_path: Option<PathBuf>,
    pub selected_target_accounts: Vec<String>,
    pub plan_summary: ApplyPlanSummary,
    pub character_mappings: Vec<CharacterMapping>,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone)]
struct PlannedEntry {
    archive_name: String,
    destination: PathBuf,
    rewrites: Vec<CharacterMapping>,
    group: ApplyGroup,
    wtf_scope: Option<WtfScope>,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
}

#[derive(Debug, Clone)]
struct PlannedCleanup {
    group: ApplyGroup,
    destination: PathBuf,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
}

struct PreparedBundleApply {
    plan: BundleApplyPlan,
    execution_operations: Vec<PreparedApplyOperation>,
}

struct BundleReader<'a> {
    bundle_path: &'a Path,
}

struct BundleReadModel {
    inspection: BundleInspection,
    entry_names: Vec<String>,
}

struct BundlePlanner<'a> {
    bundle_path: &'a Path,
    installation: &'a DetectedFlavorInstallation,
    apply_mappings: &'a BundleApplyMappings,
}

struct BundleExecution {
    backup_path: Option<PathBuf>,
    written_files: usize,
    rewritten_files: usize,
}

struct BundleExecutor<'a> {
    installation: &'a DetectedFlavorInstallation,
    backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BundleApplyMappings {
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
    #[serde(default)]
    pub selected_accounts: Vec<String>,
    #[serde(default)]
    pub all_accounts: bool,
    #[serde(default)]
    pub characters: Vec<CharacterMappingOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterMappingOverride {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: Option<String>,
    pub target_server: String,
    pub target_character: String,
}
