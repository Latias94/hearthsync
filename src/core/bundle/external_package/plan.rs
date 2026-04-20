use super::normalized::build_external_package_entry_source_map;
use super::prepare::prepare_external_package_artifacts;
use super::projection::project_external_package_plan;
use super::{ExternalPackageApplyPlan, PlanExternalPackageApplyRequest};
use crate::core::bundle::PreparedApplySource;
use crate::core::error::AppResult;

pub fn plan_external_package_apply(
    request: PlanExternalPackageApplyRequest,
) -> AppResult<ExternalPackageApplyPlan> {
    let (analysis, manifest) = prepare_external_package_artifacts(&request.external_package)?;
    let entry_source_map = build_external_package_entry_source_map(&analysis)?;
    let source_path = analysis.source_path.clone();
    let source = PreparedApplySource::ExternalPackage {
        source_path: source_path.clone(),
        source_kind: analysis.source_kind,
        entry_source_map,
    };
    let plan = super::super::planner::plan_apply_from_source(
        &source_path,
        &request.installation,
        manifest,
        &request.apply_mappings,
        &source,
    )?;

    Ok(project_external_package_plan(analysis, plan))
}
