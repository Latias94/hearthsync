use crate::core::app::{AddonLockInspectionResult, AddonLockWriteResult};

pub(in crate::cli) fn render_addon_lock_inspection(item: &AddonLockInspectionResult) -> String {
    let packages = item
        .packages
        .iter()
        .map(|package| {
            format!(
                "{} {} => {} ({})",
                package.package_id,
                package.version.as_deref().unwrap_or("unknown"),
                package.addon_directories.join(", "),
                package.content_sha256
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "Lock: {}\nGenerated: {}\nPackages: {}\n{}",
        item.lock_path.display(),
        item.generated_at,
        item.package_count,
        if packages.is_empty() {
            "none".to_string()
        } else {
            packages
        }
    )
}

pub(in crate::cli) fn render_addon_lock_write(item: &AddonLockWriteResult) -> String {
    if item.removed {
        format!(
            "Removed addon lock: {}\nTracked packages: 0",
            item.lock_path.display()
        )
    } else {
        format!(
            "Wrote addon lock: {}\nTracked packages: {}",
            item.lock_path.display(),
            item.package_count
        )
    }
}
