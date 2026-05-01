use crate::core::app::{
    AddonLockPackageDiffResult, AddonLockPackageSnapshotResult, AddonLockVerifyResult,
};

pub(super) fn push_changed_packages(
    lines: &mut Vec<String>,
    heading: &str,
    packages: &[AddonLockPackageDiffResult],
) {
    if packages.is_empty() {
        return;
    }

    lines.push(heading.to_string());
    for package in packages {
        let changed_fields = package
            .changes
            .iter()
            .map(|change| change.field.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "- {} ({})",
            addon_lock_package_label(package.left.name.as_deref(), &package.left.package_id),
            changed_fields
        ));
    }
}

pub(super) fn push_snapshot_packages(
    lines: &mut Vec<String>,
    heading: &str,
    packages: &[AddonLockPackageSnapshotResult],
) {
    if packages.is_empty() {
        return;
    }

    lines.push(heading.to_string());
    for package in packages {
        lines.push(format!(
            "- {}",
            addon_lock_package_label(package.name.as_deref(), &package.package_id)
        ));
    }
}

pub(super) fn addon_lock_package_label(name: Option<&str>, package_id: &str) -> String {
    name.unwrap_or(package_id).to_string()
}

pub(super) fn format_addon_lock_verification_summary(item: &AddonLockVerifyResult) -> String {
    if item.matches {
        "Verification: matches".to_string()
    } else {
        format!(
            "Verification: drift remains ({} changed, {} added, {} removed)",
            item.diff.changed_packages.len(),
            item.diff.added_packages.len(),
            item.diff.removed_packages.len()
        )
    }
}
