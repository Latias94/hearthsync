use super::*;

#[test]
fn diff_addon_locks_reports_changed_added_and_removed_packages() {
    let temp = tempdir().expect("temp dir");
    let left_path = temp.path().join("left.toml");
    let right_path = temp.path().join("right.toml");

    fs::write(
        &left_path,
        r#"
schema_version = 1
generated_at = "2026-04-15T00:00:00Z"

[[packages]]
package_id = "details"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/details.zip" }
content_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "omen"
name = "Omen"
source = { kind = "http_archive", url = "https://example.invalid/omen.zip" }
content_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Omen"]
addons = []
"#,
    )
    .expect("left lock");

    fs::write(
        &right_path,
        r#"
schema_version = 1
generated_at = "2026-04-16T00:00:00Z"

[[packages]]
package_id = "details-v2"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "2.0.0"
source = { kind = "http_archive", url = "https://example.invalid/details-v2.zip" }
content_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "bigwigs"
name = "BigWigs"
source = { kind = "http_archive", url = "https://example.invalid/bigwigs.zip" }
content_sha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["BigWigs"]
addons = []
"#,
    )
    .expect("right lock");

    let diff = diff_addon_locks(&left_path, &right_path).expect("diff locks");

    assert!(!diff.identical);
    assert_eq!(diff.changed_packages.len(), 1);
    assert_eq!(diff.added_packages.len(), 1);
    assert_eq!(diff.removed_packages.len(), 1);
    assert!(
        diff.changed_packages[0]
            .changes
            .iter()
            .any(|change| change.field == "version")
    );
}

#[test]
fn verify_addon_lock_reports_drift_and_untracked_addons() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    fs::write(
        installation.addon_dir.join("Details").join("Details.toc"),
        "## Interface: 110000\n## Version: 2.0.0\n",
    )
    .expect("mutate toc");
    fs::create_dir_all(installation.addon_dir.join("BigWigs")).expect("untracked addon dir");
    fs::write(
        installation.addon_dir.join("BigWigs").join("BigWigs.toc"),
        "## Interface: 110000\n## Version: 1.0.0\n",
    )
    .expect("untracked addon toc");

    let verification = verify_addon_lock(&installation, &addon_state_paths(&installation), None)
        .expect("verify lock");

    assert!(!verification.matches);
    assert_eq!(verification.diff.changed_packages.len(), 1);
    assert_eq!(verification.untracked_addons, vec!["BigWigs"]);
    assert!(
        verification.diff.changed_packages[0]
            .changes
            .iter()
            .any(|change| change.field == "content_sha256")
    );
}

#[test]
fn verify_addon_lock_treats_case_only_live_directory_as_present_on_macos() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation_for_platform(temp.path(), HostPlatform::MacOs);
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");
    let lock_path = write_addon_lock(&installation, &addon_state_paths(&installation))
        .expect("write addon lock")
        .lock_path;

    let original = installation.addon_dir.join("Details");
    let temporary = installation.addon_dir.join("__details_case_tmp");
    let lowercase = installation.addon_dir.join("details");
    fs::rename(&original, &temporary).expect("rename to temporary");
    fs::rename(&temporary, &lowercase).expect("rename to lowercase");

    let verification = verify_addon_lock(
        &installation,
        &addon_state_paths(&installation),
        Some(&lock_path),
    )
    .expect("verify addon lock");

    assert!(verification.missing_addon_directories.is_empty());
    assert!(verification.untracked_addons.is_empty());
}
