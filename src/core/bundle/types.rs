use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::addon::lock::{AddonLockApplyResult, AddonLockPlanResult};
use crate::core::install::{DetectedFlavorInstallation, LocalWowAccount};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{BundleManifest, ResourceApplyPolicy};

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
