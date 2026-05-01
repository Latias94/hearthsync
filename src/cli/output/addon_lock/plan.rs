use crate::core::app::{AddonLockPlanResult, BundleAddonLockPlanResult};

pub(in crate::cli) fn render_addon_lock_plan(item: &AddonLockPlanResult) -> String {
    render_addon_lock_plan_summary(&format!("Lock: {}", item.lock_path.display()), item)
}

pub(in crate::cli) fn render_bundle_addon_lock_plan(item: &BundleAddonLockPlanResult) -> String {
    render_addon_lock_plan_summary(
        &format!("Bundle: {}", item.bundle_path.display()),
        &item.plan,
    )
}

fn render_addon_lock_plan_summary(header: &str, item: &AddonLockPlanResult) -> String {
    let mut lines = vec![
        header.to_string(),
        format!("Embedded/lock path: {}", item.lock_path.display()),
        format!("Installation: {}", item.installation_root.display()),
        format!(
            "Summary: {} install, {} update, {} remove, {} metadata-only, {} unchanged, {} blocked",
            item.install_count,
            item.update_count,
            item.remove_count,
            item.metadata_only_count,
            item.unchanged_count,
            item.blocked_count
        ),
    ];

    if !item.untracked_addons.is_empty() {
        lines.push(format!(
            "Untracked addon directories: {}",
            item.untracked_addons.join(", ")
        ));
    }

    if item.actions.is_empty() {
        lines.push("No sync actions required.".to_string());
        return lines.join("\n");
    }

    lines.push("Actions:".to_string());
    for action in &item.actions {
        let reason = if action.reasons.is_empty() {
            "no details".to_string()
        } else {
            action.reasons.join("; ")
        };
        let mut suffix = String::new();
        if action.requires_replace_existing {
            suffix.push_str(" | requires --replace-existing");
        }
        if !action.blocked_reasons.is_empty() {
            suffix.push_str(&format!(
                " | blocked: {}",
                action.blocked_reasons.join("; ")
            ));
        }
        lines.push(format!(
            "- {:?}: {} ({}){}",
            action.kind, action.package_id, reason, suffix
        ));
    }

    lines.join("\n")
}
