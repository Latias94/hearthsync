use std::path::PathBuf;

use serde::Serialize;

use crate::core::app::{
    ApplyActionValue, ApplyGroupValue, BundleCharacterResourceValue, BundleManifestValue,
    BundleMappingRulesValue, BundlePackageValue, BundleSourceValue, HelperStrategyValue,
    ResourceApplyPolicyValue, WtfScopeValue,
};
use crate::core::bundle::{
    ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary,
    BundleAddonLockApply as DomainBundleAddonLockApply,
    BundleAddonLockPlan as DomainBundleAddonLockPlan, BundleApplyPlan as DomainBundleApplyPlan,
    BundleEntryCounts, BundleInspection, CreatedBundle as DomainCreatedBundle, GroupPolicy,
    UnpackedBundle as DomainUnpackedBundle,
};
use crate::core::install::{LocalWowAccount, LocalWowCharacter};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::BundleResources;

use super::super::map_owned_vec;
use super::addon_lock::{AddonLockApplyResult, AddonLockPlanResult};

pub type BundlePackageResult = BundlePackageValue;
pub type BundleSourceResult = BundleSourceValue;
pub type BundleCharacterResourceResult = BundleCharacterResourceValue;

#[derive(Debug, Clone, Serialize)]
pub struct BundleResourcesResult {
    pub addons: Vec<String>,
    pub addon_count: usize,
    pub wtf_common: bool,
    pub wtf_character_count: usize,
    pub wtf_characters: Vec<BundleCharacterResourceResult>,
    pub fonts: bool,
    pub interface_assets: Vec<String>,
    pub interface_asset_count: usize,
    pub addon_lock: bool,
    pub addon_indexes: Vec<String>,
}

impl BundleResourcesResult {
    pub(crate) fn from_domain(value: BundleResources) -> Self {
        let addon_count = value.addons.len();
        let wtf_character_count = value.wtf_characters.len();
        let interface_asset_count = value.interface_assets.len();

        Self {
            addons: value.addons,
            addon_count,
            wtf_common: value.wtf_common,
            wtf_character_count,
            wtf_characters: map_owned_vec(
                value.wtf_characters,
                BundleCharacterResourceResult::from_domain,
            ),
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            interface_asset_count,
            addon_lock: value.addon_lock,
            addon_indexes: value.addon_indexes,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleEntryCountsResult {
    pub total_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub metadata: usize,
}

impl BundleEntryCountsResult {
    pub(crate) fn from_domain(value: BundleEntryCounts) -> Self {
        Self {
            total_files: value.total_files,
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters,
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            metadata: value.metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleInspectionResult {
    pub archive_path: PathBuf,
    pub package: BundlePackageResult,
    pub source: BundleSourceResult,
    pub resources: BundleResourcesResult,
    pub entries: BundleEntryCountsResult,
}

impl BundleInspectionResult {
    pub(crate) fn from_domain(value: BundleInspection) -> Self {
        let package = BundlePackageResult::from_domain(value.manifest.package);
        let source = BundleSourceResult::from_domain(value.manifest.source);
        let resources = BundleResourcesResult::from_domain(value.manifest.resources);

        Self {
            archive_path: value.archive_path,
            package,
            source,
            resources,
            entries: BundleEntryCountsResult::from_domain(value.entries),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatedBundleResult {
    pub archive_path: PathBuf,
    pub archived_files: usize,
    pub manifest: BundleManifestResult,
}

impl CreatedBundleResult {
    pub(crate) fn from_domain(value: DomainCreatedBundle) -> Self {
        Self {
            archive_path: value.archive_path,
            archived_files: value.archived_files,
            manifest: BundleManifestResult::from_domain(value.manifest),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowCharacterResult {
    pub server: String,
    pub character: String,
    pub character_dir: PathBuf,
}

impl LocalWowCharacterResult {
    pub(crate) fn from_domain(value: LocalWowCharacter) -> Self {
        Self {
            server: value.server,
            character: value.character,
            character_dir: value.character_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowAccountResult {
    pub account_name: String,
    pub account_dir: PathBuf,
    pub saved_variables_dir: PathBuf,
    pub characters: Vec<LocalWowCharacterResult>,
}

impl LocalWowAccountResult {
    pub(crate) fn from_domain(value: LocalWowAccount) -> Self {
        Self {
            account_name: value.account_name,
            account_dir: value.account_dir,
            saved_variables_dir: value.saved_variables_dir,
            characters: map_owned_vec(value.characters, LocalWowCharacterResult::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CharacterMappingResult {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: String,
    pub target_server: String,
    pub target_character: String,
}

impl CharacterMappingResult {
    pub(crate) fn from_domain(value: CharacterMapping) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }
}

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

pub type BundleMappingRulesResult = BundleMappingRulesValue;
pub type BundleManifestResult = BundleManifestValue;

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

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockPlanResult {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub plan: AddonLockPlanResult,
}

impl BundleAddonLockPlanResult {
    pub(crate) fn from_domain(value: DomainBundleAddonLockPlan) -> Self {
        Self {
            bundle_path: value.bundle_path,
            embedded_lock_entry: value.embedded_lock_entry,
            plan: AddonLockPlanResult::from_domain(value.plan),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockApplyResult {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub apply: AddonLockApplyResult,
}

impl BundleAddonLockApplyResult {
    pub(crate) fn from_domain(value: DomainBundleAddonLockApply) -> Self {
        Self {
            bundle_path: value.bundle_path,
            embedded_lock_entry: value.embedded_lock_entry,
            apply: AddonLockApplyResult::from_domain(value.apply),
        }
    }
}
