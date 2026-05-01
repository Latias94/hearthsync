use crate::core::app::{AddonLockApplyResult, BundleAddonLockApplyResult};

use super::shared::format_addon_lock_verification_summary;

pub(in crate::cli) fn render_addon_lock_apply(item: &AddonLockApplyResult) -> String {
    render_addon_lock_apply_summary(
        vec![
            format!("Lock: {}", item.lock_path.display()),
            format!("Installation: {}", item.installation_root.display()),
            format!(
                "Applied: {} install, {} update, {} remove, {} metadata-only, {} unchanged",
                item.install_count,
                item.update_count,
                item.remove_count,
                item.metadata_only_count,
                item.unchanged_count
            ),
        ],
        item,
    )
}

pub(in crate::cli) fn render_bundle_addon_lock_apply(item: &BundleAddonLockApplyResult) -> String {
    render_addon_lock_apply_summary(
        vec![
            format!("Bundle: {}", item.bundle_path.display()),
            format!("Embedded lock: {}", item.embedded_lock_entry),
            format!("Installation: {}", item.apply.installation_root.display()),
            format!(
                "Applied: {} install, {} update, {} remove, {} metadata-only, {} unchanged",
                item.apply.install_count,
                item.apply.update_count,
                item.apply.remove_count,
                item.apply.metadata_only_count,
                item.apply.unchanged_count
            ),
        ],
        &item.apply,
    )
}

fn render_addon_lock_apply_summary(mut lines: Vec<String>, item: &AddonLockApplyResult) -> String {
    if !item.untracked_addons.is_empty() {
        lines.push(format!(
            "Untracked addon directories remain: {}",
            item.untracked_addons.join(", ")
        ));
    }

    lines.push(format_addon_lock_verification_summary(&item.verification));
    lines.join("\n")
}
