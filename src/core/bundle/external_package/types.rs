use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::core::bundle::types::apply::{
    ApplyGroup, ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary, BundleApplyMappings, WtfScope,
};
use crate::core::bundle::types::archive::CreatedBundle;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, LocalWowAccount, WowFlavor};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{ApplyDefaults, BundleManifest, BundleResources};

#[derive(Debug, Clone)]
pub struct AnalyzeExternalPackageRequest {
    pub source_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CreateExternalPackageBundleRequest {
    pub source_path: PathBuf,
    pub source_flavor: WowFlavor,
    pub source_platform: Option<HostPlatform>,
    pub supported_targets: Vec<WowFlavor>,
    pub output_path: Option<PathBuf>,
    pub package_id: Option<String>,
    pub package_name: Option<String>,
    pub created_by: Option<String>,
    pub description: Option<String>,
    pub apply_defaults: Option<ApplyDefaults>,
}

#[derive(Debug, Clone)]
pub struct PlanExternalPackageApplyRequest {
    pub external_package: CreateExternalPackageBundleRequest,
    pub installation: DetectedFlavorInstallation,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone)]
pub struct ApplyExternalPackageRequest {
    pub external_package: CreateExternalPackageBundleRequest,
    pub installation: DetectedFlavorInstallation,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageSourceKind {
    Directory,
    ZipArchive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCategory {
    Addon,
    Wtf,
}

impl ExternalPackageWarningCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Addon => "addon",
            Self::Wtf => "wtf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCode {
    AddonRootNotDetected,
    UnsupportedWtfLayout,
    #[serde(rename = "unsupported_wtf_root_savedvariables")]
    UnsupportedWtfRootSavedVariables,
    WtfAccountPathWithoutFile,
    #[serde(rename = "wtf_savedvariables_path_without_file")]
    WtfSavedVariablesPathWithoutFile,
    UnsupportedWtfNestedAccountLayout,
}

impl ExternalPackageWarningCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AddonRootNotDetected => "addon_root_not_detected",
            Self::UnsupportedWtfLayout => "unsupported_wtf_layout",
            Self::UnsupportedWtfRootSavedVariables => "unsupported_wtf_root_savedvariables",
            Self::WtfAccountPathWithoutFile => "wtf_account_path_without_file",
            Self::WtfSavedVariablesPathWithoutFile => "wtf_savedvariables_path_without_file",
            Self::UnsupportedWtfNestedAccountLayout => "unsupported_wtf_nested_account_layout",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExternalPackageWarning {
    pub category: ExternalPackageWarningCategory,
    pub code: ExternalPackageWarningCode,
    pub source_path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ExternalPackageWarningGroup {
    pub category: ExternalPackageWarningCategory,
    pub code: ExternalPackageWarningCode,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageAnalysis {
    pub source_path: PathBuf,
    pub source_kind: ExternalPackageSourceKind,
    pub package_id: String,
    pub package_name: String,
    pub entries: Vec<ExternalPackageEntry>,
    pub resources: BundleResources,
    pub summary: ExternalPackageSummary,
    pub warnings: Vec<ExternalPackageWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageEntry {
    pub source_path: String,
    pub normalized_path: String,
    pub group: ApplyGroup,
    pub wtf_scope: Option<WtfScope>,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExternalPackageSummary {
    pub total_files: usize,
    pub normalized_files: usize,
    pub ignored_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub warning_count: usize,
    pub addon_warning_count: usize,
    pub wtf_warning_count: usize,
    pub warning_groups: Vec<ExternalPackageWarningGroup>,
}

#[derive(Debug)]
pub struct PreparedExternalPackageBundle {
    pub analysis: ExternalPackageAnalysis,
    pub manifest: BundleManifest,
    pub bundle: CreatedBundle,
    pub(super) _stage_dir: TempDir,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageApplyPlan {
    pub analysis: ExternalPackageAnalysis,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<LocalWowAccount>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<CharacterMapping>,
    pub operations: Vec<ApplyOperation>,
    pub summary: ApplyPlanSummary,
    pub group_policies: ApplyGroupPolicies,
    pub manifest: BundleManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppliedExternalPackage {
    pub analysis: ExternalPackageAnalysis,
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
