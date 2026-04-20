use super::super::{BundleApplyPlan, UnpackedBundle};
use super::executor::BundleExecution;

pub(super) fn project_dry_run_result(plan: BundleApplyPlan) -> UnpackedBundle {
    let planned_files = plan.operations.len();

    UnpackedBundle {
        bundle_path: plan.bundle_path,
        target_flavor_root: plan.target_flavor_root,
        dry_run: true,
        planned_files,
        written_files: 0,
        rewritten_files: 0,
        backup_path: None,
        selected_target_accounts: plan.selected_target_accounts,
        plan_summary: plan.summary,
        character_mappings: plan.character_mappings,
        manifest: plan.manifest,
    }
}

pub(super) fn project_executed_result(
    plan: BundleApplyPlan,
    execution: BundleExecution,
) -> UnpackedBundle {
    let planned_files = plan.operations.len();

    UnpackedBundle {
        bundle_path: plan.bundle_path,
        target_flavor_root: plan.target_flavor_root,
        dry_run: false,
        planned_files,
        written_files: execution.written_files,
        rewritten_files: execution.rewritten_files,
        backup_path: execution.backup_path,
        selected_target_accounts: plan.selected_target_accounts,
        plan_summary: plan.summary,
        character_mappings: plan.character_mappings,
        manifest: plan.manifest,
    }
}
