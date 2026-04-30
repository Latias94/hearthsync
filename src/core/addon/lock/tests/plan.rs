use super::*;

#[test]
fn plan_addon_lock_sync_requires_replace_for_case_folded_untracked_addon_on_macos() {
    let temp = tempdir().expect("temp dir");
    let desired_lock = desired_lock_with_single_addon(
        temp.path(),
        HostPlatform::MacOs,
        "Details",
        "Details/Details.toc",
    );
    let current_installation =
        create_fixture_installation_for_platform(&temp.path().join("current"), HostPlatform::MacOs);
    create_untracked_addon(&current_installation, "details");

    let plan = plan_addon_lock_sync(
        &current_installation,
        &addon_state_paths(&current_installation),
        Some(&desired_lock),
    )
    .expect("plan");

    assert_eq!(plan.install_count, 1);
    assert_eq!(plan.blocked_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(plan.actions[0].requires_replace_existing);
}

#[test]
fn plan_addon_lock_sync_allows_case_distinct_untracked_addon_on_linux() {
    let temp = tempdir().expect("temp dir");
    let desired_lock = desired_lock_with_single_addon(
        temp.path(),
        HostPlatform::Linux,
        "Details",
        "Details/Details.toc",
    );
    let current_installation =
        create_fixture_installation_for_platform(&temp.path().join("current"), HostPlatform::Linux);
    create_untracked_addon(&current_installation, "details");

    let plan = plan_addon_lock_sync(
        &current_installation,
        &addon_state_paths(&current_installation),
        Some(&desired_lock),
    )
    .expect("plan");

    assert_eq!(plan.install_count, 1);
    assert_eq!(plan.blocked_count, 0);
    assert_eq!(plan.actions.len(), 1);
    assert!(!plan.actions[0].requires_replace_existing);
}

#[test]
fn plan_addon_lock_sync_rejects_relative_local_archive_sources() {
    let temp = tempdir().expect("temp dir");
    let desired_lock = desired_lock_with_single_addon(
        temp.path(),
        HostPlatform::Windows,
        "Details",
        "Details/Details.toc",
    );
    rewrite_first_lock_package_source_to_relative_local_archive(&desired_lock);
    let current_installation = create_fixture_installation_for_platform(
        &temp.path().join("current"),
        HostPlatform::Windows,
    );

    let error = plan_addon_lock_sync(
        &current_installation,
        &addon_state_paths(&current_installation),
        Some(&desired_lock),
    )
    .expect_err("relative local archive source should fail before planning");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("must be absolute"));
}
