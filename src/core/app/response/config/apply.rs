use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::HelperStrategyValue;

use super::super::bundle::{
    ApplyGroupPoliciesResult, ApplyOperationResult, ApplyPlanSummaryResult, BundleManifestResult,
    CharacterMappingResult, LocalWowAccountResult,
};
use super::super::external_package::{ExternalPackageApplyPlanResult, ExternalPackageApplyResult};
use super::inspection::ConfigInspectionResult;

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
