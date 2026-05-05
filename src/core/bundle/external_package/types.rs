use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use crate::core::bundle::types::apply::{
    ApplyGroup, ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary, BundleApplyMappings,
    WtfScope, WtfScopeRisk,
};
use crate::core::bundle::types::archive::CreatedBundle;
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, LocalWowAccount, WowFlavor};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{ApplyDefaults, BundleManifest, BundleResources};

#[derive(Debug, Clone)]
pub struct AnalyzeExternalPackageRequest {
    pub source_path: PathBuf,
    pub layout: ExternalPackageLayout,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateExternalPackageBundleRequest {
    pub source_path: PathBuf,
    pub layout: ExternalPackageLayout,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
    pub source_flavor: WowFlavor,
    pub source_platform: Option<HostPlatform>,
    pub supported_targets: Vec<WowFlavor>,
    pub output_path: Option<PathBuf>,
    pub package_id: Option<String>,
    pub package_name: Option<String>,
    pub created_by: Option<String>,
    pub description: Option<String>,
    pub apply_defaults: Option<ApplyDefaults>,
    pub sharing_mode: ExternalPackageSharingMode,
    pub allow_public_sharing_risks: bool,
    pub excluded_wtf_scopes: Vec<WtfScope>,
}

impl AnalyzeExternalPackageRequest {
    pub fn new(source_path: PathBuf) -> Self {
        Self {
            source_path,
            layout: ExternalPackageLayout::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
        }
    }
}

impl CreateExternalPackageBundleRequest {
    pub fn analysis_request(&self) -> AnalyzeExternalPackageRequest {
        AnalyzeExternalPackageRequest {
            source_path: self.source_path.clone(),
            layout: self.layout,
            source_account: self.source_account.clone(),
            source_server: self.source_server.clone(),
            source_character: self.source_character.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageLayout {
    #[default]
    Auto,
    Generic,
    #[serde(rename = "newbeebox_addon")]
    NewBeeBoxAddon,
    #[serde(rename = "newbeebox_font")]
    NewBeeBoxFont,
    #[serde(rename = "newbeebox_material")]
    NewBeeBoxMaterial,
    #[serde(rename = "newbeebox_wtf_account")]
    NewBeeBoxWtfAccount,
    #[serde(rename = "newbeebox_wtf_character")]
    NewBeeBoxWtfCharacter,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageSharingMode {
    #[default]
    Private,
    Public,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExternalPackageWtfScopeSummary {
    pub scope: WtfScope,
    pub risk: WtfScopeRisk,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackagePublicSharingStatus {
    Ready,
    Advisory,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackagePublicSharingSeverity {
    Advisory,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageSensitiveWtfFileKind {
    SavedVariables,
    ChatCache,
    Macros,
    Bindings,
    GameConfig,
    AddonEnablement,
    LayoutState,
}

impl ExternalPackageSensitiveWtfFileKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SavedVariables => "saved_variables",
            Self::ChatCache => "chat_cache",
            Self::Macros => "macros",
            Self::Bindings => "bindings",
            Self::GameConfig => "game_config",
            Self::AddonEnablement => "addon_enablement",
            Self::LayoutState => "layout_state",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ExternalPackageSensitiveWtfFileSummary {
    pub kind: ExternalPackageSensitiveWtfFileKind,
    pub severity: ExternalPackagePublicSharingSeverity,
    pub count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackagePublicSharingReasonCode {
    NormalizationWarnings,
    HighRiskWtfScope,
    MediumRiskWtfScope,
    LowRiskWtfScope,
    UnknownRiskWtfScope,
    SensitiveWtfFile,
    AdvisoryWtfFile,
    SourceAccountIdentity,
    SourceCharacterIdentity,
}

impl ExternalPackagePublicSharingReasonCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NormalizationWarnings => "normalization_warnings",
            Self::HighRiskWtfScope => "high_risk_wtf_scope",
            Self::MediumRiskWtfScope => "medium_risk_wtf_scope",
            Self::LowRiskWtfScope => "low_risk_wtf_scope",
            Self::UnknownRiskWtfScope => "unknown_risk_wtf_scope",
            Self::SensitiveWtfFile => "sensitive_wtf_file",
            Self::AdvisoryWtfFile => "advisory_wtf_file",
            Self::SourceAccountIdentity => "source_account_identity",
            Self::SourceCharacterIdentity => "source_character_identity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ExternalPackagePublicSharingReason {
    pub severity: ExternalPackagePublicSharingSeverity,
    pub code: ExternalPackagePublicSharingReasonCode,
    pub count: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExternalPackagePublicSharingSummary {
    pub status: ExternalPackagePublicSharingStatus,
    pub public_ready: bool,
    pub review_required_count: usize,
    pub advisory_count: usize,
    pub reasons: Vec<ExternalPackagePublicSharingReason>,
}

impl Default for ExternalPackagePublicSharingSummary {
    fn default() -> Self {
        Self {
            status: ExternalPackagePublicSharingStatus::Ready,
            public_ready: true,
            review_required_count: 0,
            advisory_count: 0,
            reasons: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct ExternalPackageSourceCharacterSummary {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ExternalPackageSourceIdentitySummary {
    pub source_accounts: Vec<String>,
    pub source_characters: Vec<ExternalPackageSourceCharacterSummary>,
    pub entries_with_source_account: usize,
    pub entries_with_source_character: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageAnalysis {
    pub source_path: PathBuf,
    pub source_kind: ExternalPackageSourceKind,
    pub layout: ExternalPackageLayout,
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
    pub wtf_scopes: Vec<ExternalPackageWtfScopeSummary>,
    pub sensitive_wtf_files: Vec<ExternalPackageSensitiveWtfFileSummary>,
    pub source_identities: ExternalPackageSourceIdentitySummary,
    pub public_sharing: ExternalPackagePublicSharingSummary,
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
