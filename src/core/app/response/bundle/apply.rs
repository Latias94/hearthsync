use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::{
    ApplyActionValue, ApplyGroupValue, HelperStrategyValue, ResourceApplyPolicyValue, WtfScopeValue,
};
use crate::core::bundle::{
    ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary, BundleApplyPlan as DomainBundleApplyPlan,
    GroupPolicy, UnpackedBundle as DomainUnpackedBundle,
};

use super::super::super::map_owned_vec;
use super::local::{CharacterMappingResult, LocalWowAccountResult};
use super::manifest::BundleManifestResult;

#[derive(Debug, Clone, Serialize)]
pub struct ApplyOperationResult {
    pub group: ApplyGroupValue,
    pub wtf_scope: Option<WtfScopeValue>,
    pub action: ApplyActionValue,
    pub archive_name: String,
    pub destination: PathBuf,
    pub target_account: Option<String>,
    pub target_server: Option<String>,
    pub target_character: Option<String>,
}

impl ApplyOperationResult {
    pub(crate) fn from_domain(value: ApplyOperation) -> Self {
        Self {
            group: ApplyGroupValue::from_domain(value.group),
            wtf_scope: value.wtf_scope.map(WtfScopeValue::from_domain),
            action: ApplyActionValue::from_domain(value.action),
            archive_name: value.archive_name,
            destination: value.destination,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyPlanSummaryResult {
    pub files_to_add: usize,
    pub files_to_replace: usize,
    pub files_to_skip: usize,
    pub paths_to_remove: usize,
    pub files_to_preserve: usize,
}

impl ApplyPlanSummaryResult {
    pub(crate) fn from_domain(value: ApplyPlanSummary) -> Self {
        Self {
            files_to_add: value.files_to_add,
            files_to_replace: value.files_to_replace,
            files_to_skip: value.files_to_skip,
            paths_to_remove: value.paths_to_remove,
            files_to_preserve: value.files_to_preserve,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupPolicyResult {
    pub policy: ResourceApplyPolicyValue,
}

impl GroupPolicyResult {
    pub(crate) fn from_domain(value: GroupPolicy) -> Self {
        Self {
            policy: ResourceApplyPolicyValue::from_domain(value.policy),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyGroupPoliciesResult {
    pub addons: GroupPolicyResult,
    pub wtf_common: GroupPolicyResult,
    pub wtf_characters: GroupPolicyResult,
    pub fonts: GroupPolicyResult,
    pub interface_assets: GroupPolicyResult,
    pub metadata: GroupPolicyResult,
}

impl ApplyGroupPoliciesResult {
    pub(crate) fn from_domain(value: ApplyGroupPolicies) -> Self {
        Self {
            addons: GroupPolicyResult::from_domain(value.addons),
            wtf_common: GroupPolicyResult::from_domain(value.wtf_common),
            wtf_characters: GroupPolicyResult::from_domain(value.wtf_characters),
            fonts: GroupPolicyResult::from_domain(value.fonts),
            interface_assets: GroupPolicyResult::from_domain(value.interface_assets),
            metadata: GroupPolicyResult::from_domain(value.metadata),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleApplyPlanResult {
    pub bundle_path: PathBuf,
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

impl BundleApplyPlanResult {
    pub(crate) fn from_domain_plan(
        value: DomainBundleApplyPlan,
        helper_strategy: HelperStrategyValue,
    ) -> Self {
        Self {
            bundle_path: value.bundle_path,
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
pub struct BundleApplyResult {
    pub bundle_path: PathBuf,
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

impl BundleApplyResult {
    pub(crate) fn from_domain(value: DomainUnpackedBundle) -> Self {
        Self {
            bundle_path: value.bundle_path,
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
