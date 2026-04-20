use std::path::PathBuf;

use serde::Serialize;

use super::map_domain_vec;
use crate::core::app::{
    ApplyGroupValue, ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
    HelperStrategyValue, WtfScopeValue,
};
use crate::core::bundle::{
    AppliedExternalPackage as DomainAppliedExternalPackage,
    ExternalPackageAnalysis as DomainExternalPackageAnalysis,
    ExternalPackageApplyPlan as DomainExternalPackageApplyPlan,
    ExternalPackageEntry as DomainExternalPackageEntry, ExternalPackageSourceKind,
    ExternalPackageSummary as DomainExternalPackageSummary,
    ExternalPackageWarning as DomainExternalPackageWarning,
    ExternalPackageWarningGroup as DomainExternalPackageWarningGroup,
    PreparedExternalPackageBundle as DomainPreparedExternalPackageBundle,
};

use super::bundle::{
    ApplyGroupPoliciesResult, ApplyOperationResult, ApplyPlanSummaryResult, BundleManifestResult,
    BundleResourcesResult, CharacterMappingResult, CreatedBundleResult, LocalWowAccountResult,
};

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageBundleResult {
    pub analysis: ExternalPackageAnalysisResult,
    pub manifest: BundleManifestResult,
    pub bundle: CreatedBundleResult,
}

#[derive(Debug)]
pub struct ExternalPackageBundleHandle {
    result: ExternalPackageBundleResult,
    _prepared: DomainPreparedExternalPackageBundle,
}

impl ExternalPackageBundleHandle {
    pub(crate) fn from_domain(value: DomainPreparedExternalPackageBundle) -> Self {
        let result = ExternalPackageBundleResult {
            analysis: ExternalPackageAnalysisResult::from_domain(value.analysis.clone()),
            manifest: BundleManifestResult::from_domain(value.manifest.clone()),
            bundle: CreatedBundleResult::from_domain(value.bundle.clone()),
        };

        Self {
            result,
            _prepared: value,
        }
    }
}

impl AsRef<ExternalPackageBundleResult> for ExternalPackageBundleHandle {
    fn as_ref(&self) -> &ExternalPackageBundleResult {
        &self.result
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageEntryResult {
    pub source_path: String,
    pub normalized_path: String,
    pub group: ApplyGroupValue,
    pub wtf_scope: Option<WtfScopeValue>,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

impl ExternalPackageEntryResult {
    pub(crate) fn from_domain(value: DomainExternalPackageEntry) -> Self {
        Self {
            source_path: value.source_path,
            normalized_path: value.normalized_path,
            group: ApplyGroupValue::from_domain(value.group),
            wtf_scope: value.wtf_scope.map(WtfScopeValue::from_domain),
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageWarningGroupResult {
    pub category: ExternalPackageWarningCategoryValue,
    pub code: ExternalPackageWarningCodeValue,
    pub count: usize,
}

impl ExternalPackageWarningGroupResult {
    pub(crate) fn from_domain(value: DomainExternalPackageWarningGroup) -> Self {
        Self {
            category: ExternalPackageWarningCategoryValue::from_domain(value.category),
            code: ExternalPackageWarningCodeValue::from_domain(value.code),
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageWarningResult {
    pub category: ExternalPackageWarningCategoryValue,
    pub code: ExternalPackageWarningCodeValue,
    pub source_path: String,
    pub message: String,
}

impl ExternalPackageWarningResult {
    pub(crate) fn from_domain(value: DomainExternalPackageWarning) -> Self {
        Self {
            category: ExternalPackageWarningCategoryValue::from_domain(value.category),
            code: ExternalPackageWarningCodeValue::from_domain(value.code),
            source_path: value.source_path,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageSummaryResult {
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
    pub warning_groups: Vec<ExternalPackageWarningGroupResult>,
}

impl ExternalPackageSummaryResult {
    pub(crate) fn from_domain(value: DomainExternalPackageSummary) -> Self {
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
            warning_groups: map_domain_vec(
                value.warning_groups,
                ExternalPackageWarningGroupResult::from_domain,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageAnalysisResult {
    pub source_path: PathBuf,
    pub source_kind: ExternalPackageSourceKind,
    pub package_id: String,
    pub package_name: String,
    pub entry_count: usize,
    pub entries: Vec<ExternalPackageEntryResult>,
    pub resources: BundleResourcesResult,
    pub summary: ExternalPackageSummaryResult,
    pub warnings: Vec<ExternalPackageWarningResult>,
}

impl ExternalPackageAnalysisResult {
    pub(crate) fn from_domain(value: DomainExternalPackageAnalysis) -> Self {
        let entry_count = value.entries.len();

        Self {
            source_path: value.source_path,
            source_kind: value.source_kind,
            package_id: value.package_id,
            package_name: value.package_name,
            entry_count,
            entries: map_domain_vec(value.entries, ExternalPackageEntryResult::from_domain),
            resources: BundleResourcesResult::from_domain(value.resources),
            summary: ExternalPackageSummaryResult::from_domain(value.summary),
            warnings: map_domain_vec(value.warnings, ExternalPackageWarningResult::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageApplyPlanResult {
    pub analysis: ExternalPackageAnalysisResult,
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

impl ExternalPackageApplyPlanResult {
    pub(crate) fn from_domain_plan(
        value: DomainExternalPackageApplyPlan,
        helper_strategy: HelperStrategyValue,
    ) -> Self {
        Self {
            analysis: ExternalPackageAnalysisResult::from_domain(value.analysis),
            target_flavor_root: value.target_flavor_root,
            discovered_accounts: map_domain_vec(
                value.discovered_accounts,
                LocalWowAccountResult::from_domain,
            ),
            selected_target_accounts: value.selected_target_accounts,
            character_mappings: map_domain_vec(
                value.character_mappings,
                CharacterMappingResult::from_domain,
            ),
            operations: map_domain_vec(value.operations, ApplyOperationResult::from_domain),
            summary: ApplyPlanSummaryResult::from_domain(value.summary),
            helper_strategy,
            group_policies: ApplyGroupPoliciesResult::from_domain(value.group_policies),
            manifest: BundleManifestResult::from_domain(value.manifest),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageApplyResult {
    pub analysis: ExternalPackageAnalysisResult,
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

impl ExternalPackageApplyResult {
    pub(crate) fn from_domain(value: DomainAppliedExternalPackage) -> Self {
        Self {
            analysis: ExternalPackageAnalysisResult::from_domain(value.analysis),
            target_flavor_root: value.target_flavor_root,
            dry_run: value.dry_run,
            planned_files: value.planned_files,
            written_files: value.written_files,
            rewritten_files: value.rewritten_files,
            backup_path: value.backup_path,
            selected_target_accounts: value.selected_target_accounts,
            plan_summary: ApplyPlanSummaryResult::from_domain(value.plan_summary),
            character_mappings: map_domain_vec(
                value.character_mappings,
                CharacterMappingResult::from_domain,
            ),
            manifest: BundleManifestResult::from_domain(value.manifest),
        }
    }
}
