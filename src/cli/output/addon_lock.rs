use crate::core::app::{
    AddonLockApplyResult, AddonLockDiffResult, AddonLockInspectionResult,
    AddonLockPackageDiffResult, AddonLockPackageSnapshotResult, AddonLockPlanResult,
    AddonLockVerifyResult, AddonLockWriteResult, BundleAddonLockApplyResult,
    BundleAddonLockPlanResult,
};

pub(in crate::cli) fn render_addon_lock_plan(item: &AddonLockPlanResult) -> String {
    render_addon_lock_plan_summary(&format!("Lock: {}", item.lock_path.display()), item)
}

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

pub(in crate::cli) fn render_bundle_addon_lock_plan(item: &BundleAddonLockPlanResult) -> String {
    render_addon_lock_plan_summary(
        &format!("Bundle: {}", item.bundle_path.display()),
        &item.plan,
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

fn push_changed_packages(
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

fn push_snapshot_packages(
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

fn addon_lock_package_label(name: Option<&str>, package_id: &str) -> String {
    name.unwrap_or(package_id).to_string()
}

fn format_addon_lock_verification_summary(item: &AddonLockVerifyResult) -> String {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::super::test_support::{
        sample_addon_lock_apply, sample_addon_lock_plan, sample_snapshot,
    };
    use super::*;
    use crate::core::app::{
        AddonLockDiffResult, AddonLockFieldChangeResult, AddonLockPackageDiffResult,
        AddonLockPackageDirectoryIssueResult, AddonLockVerifyResult, BundleAddonLockApplyResult,
        BundleAddonLockPlanResult,
    };

    #[test]
    fn render_addon_lock_plan_uses_lock_header() {
        let rendered = render_addon_lock_plan(&sample_addon_lock_plan());

        assert!(rendered.contains("Lock: addon.lock"));
        assert!(rendered.contains("Installation: World of Warcraft/_retail_"));
        assert!(rendered.contains(
            "Summary: 1 install, 2 update, 3 remove, 4 metadata-only, 5 unchanged, 0 blocked"
        ));
        assert!(rendered.contains("Untracked addon directories: LooseAddon"));
        assert!(rendered.contains("No sync actions required."));
    }

    #[test]
    fn render_addon_lock_apply_includes_verification_summary() {
        let rendered = render_addon_lock_apply(&sample_addon_lock_apply());

        assert!(rendered.contains("Lock: addon.lock"));
        assert!(
            rendered
                .contains("Applied: 1 install, 2 update, 3 remove, 4 metadata-only, 5 unchanged")
        );
        assert!(rendered.contains("Verification: matches"));
    }

    #[test]
    fn render_bundle_addon_lock_plan_uses_bundle_header() {
        let rendered = render_bundle_addon_lock_plan(&BundleAddonLockPlanResult {
            bundle_path: PathBuf::from("ui.zip"),
            embedded_lock_entry: "metadata/addons/lock.toml".to_string(),
            plan: sample_addon_lock_plan(),
        });

        assert!(rendered.contains("Bundle: ui.zip"));
        assert!(rendered.contains("No sync actions required."));
    }

    #[test]
    fn render_bundle_addon_lock_apply_includes_embedded_lock() {
        let rendered = render_bundle_addon_lock_apply(&BundleAddonLockApplyResult {
            bundle_path: PathBuf::from("ui.zip"),
            embedded_lock_entry: "metadata/addons/lock.toml".to_string(),
            apply: sample_addon_lock_apply(),
        });

        assert!(rendered.contains("Bundle: ui.zip"));
        assert!(rendered.contains("Embedded lock: metadata/addons/lock.toml"));
        assert!(rendered.contains("Verification: matches"));
    }

    #[test]
    fn render_addon_lock_diff_groups_changed_added_and_removed_packages() {
        let rendered = render_addon_lock_diff(&AddonLockDiffResult {
            left_label: "left.lock".to_string(),
            right_label: "right.lock".to_string(),
            left_package_count: 1,
            right_package_count: 2,
            identical: false,
            unchanged_packages: 3,
            added_package_count: 1,
            removed_package_count: 1,
            changed_package_count: 1,
            added_packages: vec![sample_snapshot("new-package", Some("New Package"))],
            removed_packages: vec![sample_snapshot("old-package", None)],
            changed_packages: vec![AddonLockPackageDiffResult {
                comparison_key: "shared".to_string(),
                left: sample_snapshot("details", Some("Details")),
                right: sample_snapshot("details", Some("Details")),
                changes: vec![AddonLockFieldChangeResult {
                    field: "version".to_string(),
                    left: Some("1.0.0".to_string()),
                    right: Some("2.0.0".to_string()),
                }],
            }],
        });

        assert!(rendered.contains("Summary: 1 changed, 1 added, 1 removed, 3 unchanged"));
        assert!(rendered.contains("Changed packages:"));
        assert!(rendered.contains("- Details (version)"));
        assert!(rendered.contains("Added packages:"));
        assert!(rendered.contains("- New Package"));
        assert!(rendered.contains("Removed packages:"));
        assert!(rendered.contains("- old-package"));
    }

    #[test]
    fn render_addon_lock_verify_includes_missing_and_untracked_sections() {
        let rendered = render_addon_lock_verify(&AddonLockVerifyResult {
            lock_path: PathBuf::from("addon.lock"),
            installation_root: PathBuf::from("World of Warcraft/_retail_"),
            tracked_package_count: 2,
            untracked_addon_count: 1,
            untracked_addons: vec!["LooseAddon".to_string()],
            missing_package_count: 1,
            missing_addon_directories: vec![AddonLockPackageDirectoryIssueResult {
                comparison_key: "pkg".to_string(),
                package_id: "weakauras".to_string(),
                missing_addon_directories: vec!["WeakAuras".to_string()],
            }],
            diff: AddonLockDiffResult {
                left_label: "lock".to_string(),
                right_label: "install".to_string(),
                left_package_count: 2,
                right_package_count: 2,
                identical: false,
                unchanged_packages: 0,
                added_package_count: 1,
                removed_package_count: 1,
                changed_package_count: 1,
                added_packages: vec![sample_snapshot("extra", None)],
                removed_packages: vec![sample_snapshot("missing", Some("Missing Package"))],
                changed_packages: vec![AddonLockPackageDiffResult {
                    comparison_key: "details".to_string(),
                    left: sample_snapshot("details", Some("Details")),
                    right: sample_snapshot("details", Some("Details")),
                    changes: vec![AddonLockFieldChangeResult {
                        field: "content_sha256".to_string(),
                        left: Some("old".to_string()),
                        right: Some("new".to_string()),
                    }],
                }],
            },
            matches: false,
        });

        assert!(rendered.contains("Result: drift detected"));
        assert!(rendered.contains("Missing tracked addon directories:"));
        assert!(rendered.contains("- weakauras => WeakAuras"));
        assert!(rendered.contains("Untracked addon directories: LooseAddon"));
        assert!(rendered.contains("Unexpected tracked packages:"));
        assert!(rendered.contains("- extra"));
        assert!(rendered.contains("Missing expected packages:"));
        assert!(rendered.contains("- Missing Package"));
    }
}
