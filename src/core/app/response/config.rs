use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::{
    ApplyGroupValue, HelperStrategyValue, WtfScopeValue,
    response::bundle::{
        ApplyGroupPoliciesResult, ApplyOperationResult, ApplyPlanSummaryResult,
        BundleManifestResult, BundleResourcesResult, CharacterMappingResult, LocalWowAccountResult,
    },
    response::external_package::{
        ExternalPackageAnalysisResult, ExternalPackageApplyPlanResult, ExternalPackageApplyResult,
        ExternalPackageEntryResult, ExternalPackageSummaryResult,
        ExternalPackageWarningGroupResult, ExternalPackageWarningResult,
    },
    types::external_package::{
        ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
    },
};
use crate::core::bundle::ExternalPackageSourceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPackageSourceKindResult {
    Directory,
    ZipArchive,
}

impl ConfigPackageSourceKindResult {
    fn from_external(value: ExternalPackageSourceKind) -> Self {
        match value {
            ExternalPackageSourceKind::Directory => Self::Directory,
            ExternalPackageSourceKind::ZipArchive => Self::ZipArchive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigWarningCategoryValue {
    Addon,
    Wtf,
}

impl ConfigWarningCategoryValue {
    fn from_external(value: ExternalPackageWarningCategoryValue) -> Self {
        match value {
            ExternalPackageWarningCategoryValue::Addon => Self::Addon,
            ExternalPackageWarningCategoryValue::Wtf => Self::Wtf,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigWarningCodeValue {
    AddonRootNotDetected,
    UnsupportedWtfLayout,
    WtfAccountPathWithoutFile,
    WtfSavedVariablesPathWithoutFile,
    UnsupportedWtfNestedAccountLayout,
}

impl ConfigWarningCodeValue {
    fn from_external(value: ExternalPackageWarningCodeValue) -> Self {
        match value {
            ExternalPackageWarningCodeValue::AddonRootNotDetected => Self::AddonRootNotDetected,
            ExternalPackageWarningCodeValue::UnsupportedWtfLayout => Self::UnsupportedWtfLayout,
            ExternalPackageWarningCodeValue::WtfAccountPathWithoutFile => {
                Self::WtfAccountPathWithoutFile
            }
            ExternalPackageWarningCodeValue::WtfSavedVariablesPathWithoutFile => {
                Self::WtfSavedVariablesPathWithoutFile
            }
            ExternalPackageWarningCodeValue::UnsupportedWtfNestedAccountLayout => {
                Self::UnsupportedWtfNestedAccountLayout
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPackageEntryResult {
    pub source_path: String,
    pub normalized_path: String,
    pub group: ApplyGroupValue,
    pub wtf_scope: Option<WtfScopeValue>,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

impl ConfigPackageEntryResult {
    fn from_external(value: ExternalPackageEntryResult) -> Self {
        Self {
            source_path: value.source_path,
            normalized_path: value.normalized_path,
            group: value.group,
            wtf_scope: value.wtf_scope,
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigWarningGroupResult {
    pub category: ConfigWarningCategoryValue,
    pub code: ConfigWarningCodeValue,
    pub count: usize,
}

impl ConfigWarningGroupResult {
    fn from_external(value: ExternalPackageWarningGroupResult) -> Self {
        Self {
            category: ConfigWarningCategoryValue::from_external(value.category),
            code: ConfigWarningCodeValue::from_external(value.code),
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigWarningResult {
    pub category: ConfigWarningCategoryValue,
    pub code: ConfigWarningCodeValue,
    pub source_path: String,
    pub message: String,
}

impl ConfigWarningResult {
    fn from_external(value: ExternalPackageWarningResult) -> Self {
        Self {
            category: ConfigWarningCategoryValue::from_external(value.category),
            code: ConfigWarningCodeValue::from_external(value.code),
            source_path: value.source_path,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPackageSummaryResult {
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
    pub warning_groups: Vec<ConfigWarningGroupResult>,
}

impl ConfigPackageSummaryResult {
    fn from_external(value: ExternalPackageSummaryResult) -> Self {
        Self {
            total_files: value.total_files,
            normalized_files: value.normalized_files,
            ignored_files: value.ignored_files,
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters,
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            warning_count: value.warning_count,
            addon_warning_count: value.addon_warning_count,
            wtf_warning_count: value.wtf_warning_count,
            warning_groups: value
                .warning_groups
                .into_iter()
                .map(ConfigWarningGroupResult::from_external)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigInspectionResult {
    pub source_path: PathBuf,
    pub source_kind: ConfigPackageSourceKindResult,
    pub package_id: String,
    pub package_name: String,
    pub entry_count: usize,
    pub entries: Vec<ConfigPackageEntryResult>,
    pub resources: BundleResourcesResult,
    pub summary: ConfigPackageSummaryResult,
    pub warnings: Vec<ConfigWarningResult>,
}

impl ConfigInspectionResult {
    pub(crate) fn from_external(value: ExternalPackageAnalysisResult) -> Self {
        Self {
            source_path: value.source_path,
            source_kind: ConfigPackageSourceKindResult::from_external(value.source_kind),
            package_id: value.package_id,
            package_name: value.package_name,
            entry_count: value.entry_count,
            entries: value
                .entries
                .into_iter()
                .map(ConfigPackageEntryResult::from_external)
                .collect(),
            resources: value.resources,
            summary: ConfigPackageSummaryResult::from_external(value.summary),
            warnings: value
                .warnings
                .into_iter()
                .map(ConfigWarningResult::from_external)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigApplyPlanResult {
    pub inspection: ConfigInspectionResult,
    pub target_flavor_root: PathBuf,
    pub discovered_accounts: Vec<LocalWowAccountResult>,
    pub selected_target_accounts: Vec<String>,
    pub character_mappings: Vec<CharacterMappingResult>,
    pub operations: Vec<ApplyOperationResult>,
    pub summary: ApplyPlanSummaryResult,
    pub helper_strategy: HelperStrategyValue,
    pub group_policies: ApplyGroupPoliciesResult,
    pub manifest: BundleManifestResult,
}

impl ConfigApplyPlanResult {
    pub(crate) fn from_external(value: ExternalPackageApplyPlanResult) -> Self {
        Self {
            inspection: ConfigInspectionResult::from_external(value.analysis),
            target_flavor_root: value.target_flavor_root,
            discovered_accounts: value.discovered_accounts,
            selected_target_accounts: value.selected_target_accounts,
            character_mappings: value.character_mappings,
            operations: value.operations,
            summary: value.summary,
            helper_strategy: value.helper_strategy,
            group_policies: value.group_policies,
            manifest: value.manifest,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigApplyResult {
    pub inspection: ConfigInspectionResult,
    pub target_flavor_root: PathBuf,
    pub dry_run: bool,
    pub planned_files: usize,
    pub written_files: usize,
    pub rewritten_files: usize,
    pub backup_path: Option<PathBuf>,
    pub selected_target_accounts: Vec<String>,
    pub plan_summary: ApplyPlanSummaryResult,
    pub character_mappings: Vec<CharacterMappingResult>,
    pub manifest: BundleManifestResult,
}

impl ConfigApplyResult {
    pub(crate) fn from_external(value: ExternalPackageApplyResult) -> Self {
        Self {
            inspection: ConfigInspectionResult::from_external(value.analysis),
            target_flavor_root: value.target_flavor_root,
            dry_run: value.dry_run,
            planned_files: value.planned_files,
            written_files: value.written_files,
            rewritten_files: value.rewritten_files,
            backup_path: value.backup_path,
            selected_target_accounts: value.selected_target_accounts,
            plan_summary: value.plan_summary,
            character_mappings: value.character_mappings,
            manifest: value.manifest,
        }
    }
}
