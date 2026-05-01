use crate::core::app::{AddonLockDiffResult, AddonLockVerifyResult};

use super::shared::{push_changed_packages, push_snapshot_packages};

pub(in crate::cli) fn render_addon_lock_diff(item: &AddonLockDiffResult) -> String {
    let mut lines = vec![
        format!("Left: {}", item.left_label),
        format!("Right: {}", item.right_label),
        format!(
            "Summary: {} changed, {} added, {} removed, {} unchanged",
            item.changed_packages.len(),
            item.added_packages.len(),
            item.removed_packages.len(),
            item.unchanged_packages
        ),
    ];

    if item.identical {
        lines.push("Result: identical".to_string());
        return lines.join("\n");
    }

    push_changed_packages(&mut lines, "Changed packages:", &item.changed_packages);
    push_snapshot_packages(&mut lines, "Added packages:", &item.added_packages);
    push_snapshot_packages(&mut lines, "Removed packages:", &item.removed_packages);

    lines.join("\n")
}

pub(in crate::cli) fn render_addon_lock_verify(item: &AddonLockVerifyResult) -> String {
    let mut lines = vec![
        format!("Lock: {}", item.lock_path.display()),
        format!("Installation: {}", item.installation_root.display()),
        format!(
            "Summary: {} changed, {} added, {} removed, {} unchanged",
            item.diff.changed_packages.len(),
            item.diff.added_packages.len(),
            item.diff.removed_packages.len(),
            item.diff.unchanged_packages
        ),
    ];

    if item.matches {
        lines.push("Result: verified".to_string());
        return lines.join("\n");
    }

    lines.push("Result: drift detected".to_string());

    if !item.missing_addon_directories.is_empty() {
        lines.push("Missing tracked addon directories:".to_string());
        for issue in &item.missing_addon_directories {
            lines.push(format!(
                "- {} => {}",
                issue.package_id,
                issue.missing_addon_directories.join(", ")
            ));
        }
    }

    if !item.untracked_addons.is_empty() {
        lines.push(format!(
            "Untracked addon directories: {}",
            item.untracked_addons.join(", ")
        ));
    }

    push_changed_packages(&mut lines, "Changed packages:", &item.diff.changed_packages);
    push_snapshot_packages(
        &mut lines,
        "Unexpected tracked packages:",
        &item.diff.added_packages,
    );
    push_snapshot_packages(
        &mut lines,
        "Missing expected packages:",
        &item.diff.removed_packages,
    );

    lines.join("\n")
}
