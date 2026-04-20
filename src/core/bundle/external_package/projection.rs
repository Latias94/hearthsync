use super::super::types::{BundleApplyPlan, UnpackedBundle};
use super::types::{AppliedExternalPackage, ExternalPackageAnalysis, ExternalPackageApplyPlan};

pub(super) fn project_external_package_plan(
    analysis: ExternalPackageAnalysis,
    plan: BundleApplyPlan,
) -> ExternalPackageApplyPlan {
    ExternalPackageApplyPlan {
        analysis,
        target_flavor_root: plan.target_flavor_root,
        discovered_accounts: plan.discovered_accounts,
        selected_target_accounts: plan.selected_target_accounts,
        character_mappings: plan.character_mappings,
        operations: plan.operations,
        summary: plan.summary,
        group_policies: plan.group_policies,
        manifest: plan.manifest,
    }
}

pub(super) fn project_applied_external_package(
    analysis: ExternalPackageAnalysis,
    result: UnpackedBundle,
) -> AppliedExternalPackage {
    AppliedExternalPackage {
        analysis,
        target_flavor_root: result.target_flavor_root,
        dry_run: result.dry_run,
        planned_files: result.planned_files,
        written_files: result.written_files,
        rewritten_files: result.rewritten_files,
        backup_path: result.backup_path,
        selected_target_accounts: result.selected_target_accounts,
        plan_summary: result.plan_summary,
        character_mappings: result.character_mappings,
        manifest: result.manifest,
    }
}
