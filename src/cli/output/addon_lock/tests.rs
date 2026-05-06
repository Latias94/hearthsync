use std::path::PathBuf;

use super::super::test_support::{
    sample_addon_lock_apply, sample_addon_lock_plan, sample_snapshot,
};
use super::{
    render_addon_lock_apply, render_addon_lock_diff, render_addon_lock_plan,
    render_addon_lock_verify, render_bundle_addon_lock_apply, render_bundle_addon_lock_plan,
};
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
    assert!(rendered.contains("Backup: backup.zip"));
    assert!(
        rendered.contains("Applied: 1 install, 2 update, 3 remove, 4 metadata-only, 5 unchanged")
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
    assert!(rendered.contains("Backup: backup.zip"));
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
