use super::result::{attach_blocked_package_count, index_attach_result};
use super::*;

pub(super) fn execute_index_attach_plan(
    plan: IndexAttachPlan,
) -> AppResult<AddonIndexAttachResult> {
    let mut registry = load_registry(&plan.installation, &plan.state_paths)?;
    let timestamp = relink_timestamp()?;

    for change in &plan.changes {
        let target = registry
            .packages
            .get(change.tracked_package_index)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could be applied",
                    change.tracked_package.package_id
                ))
            })?;
        if *target != change.tracked_package {
            return Err(AppError::Validation(format!(
                "tracked addon package `{}` changed before addon index attach could be applied",
                change.tracked_package.package_id
            )));
        }
    }

    for change in &plan.changes {
        let target = registry
            .packages
            .get_mut(change.tracked_package_index)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked addon package `{}` disappeared before addon index attach could be applied",
                    change.tracked_package.package_id
                ))
            })?;
        target.source = change.next_source.clone();
        target.updated_at = timestamp.clone();
        target.metadata = Some(change.metadata.clone());
    }
    save_registry(&plan.installation, &plan.state_paths, &registry)?;

    Ok(index_attach_result(plan, true))
}

pub(super) fn index_attach_execute_message(plan: &IndexAttachPlan) -> String {
    let blocked_count = attach_blocked_package_count(&plan.packages);
    if blocked_count > 0 {
        format!(
            "Partially attaching {} ready curated addon index package(s) without reinstalling live AddOns; {} package(s) remain blocked",
            plan.changes.len(),
            blocked_count
        )
    } else {
        format!(
            "Attaching {} curated addon index package(s) without reinstalling live AddOns",
            plan.changes.len()
        )
    }
}
