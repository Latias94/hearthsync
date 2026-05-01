use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::HelperStrategyValue;
use crate::core::bundle::{
    AppliedExternalPackage as DomainAppliedExternalPackage,
    ExternalPackageApplyPlan as DomainExternalPackageApplyPlan,
};

use super::super::super::map_owned_vec;
use super::super::bundle::{
    ApplyGroupPoliciesResult, ApplyOperationResult, ApplyPlanSummaryResult, BundleManifestResult,
    CharacterMappingResult, LocalWowAccountResult,
};
use super::analysis::ExternalPackageAnalysisResult;

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
            discovered_accounts: map_owned_vec(
                value.discovered_accounts,
                LocalWowAccountResult::from_domain,
            ),
            selected_target_accounts: value.selected_target_accounts,
            character_mappings: map_owned_vec(
                value.character_mappings,
                CharacterMappingResult::from_domain,
            ),
            operations: map_owned_vec(value.operations, ApplyOperationResult::from_domain),
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
            character_mappings: map_owned_vec(
                value.character_mappings,
                CharacterMappingResult::from_domain,
            ),
            manifest: BundleManifestResult::from_domain(value.manifest),
        }
    }
}
