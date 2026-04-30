use std::fs;

use tempfile::tempdir;

use super::{addon_state_paths, create_fixture_installation};
use crate::core::addon::provider::AddonSourceRef;
use crate::core::addon::{AdoptAddonsRequest, adopt_addons, list_addons};
use crate::core::error::AppError;

#[test]
fn adopt_addons_writes_snapshot_archive_and_registry_for_explicit_untracked_addon() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let addon_dir = installation.addon_dir.join("Plater");
    fs::create_dir_all(&addon_dir).expect("plater dir");
    fs::write(
        addon_dir.join("Plater.toc"),
        "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
    )
    .expect("write toc");
    fs::write(addon_dir.join("Core.lua"), "print('plater')").expect("write lua");

    let result = adopt_addons(AdoptAddonsRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        addon_directories: vec!["Plater".to_string()],
        package_id: None,
        archive_output_path: None,
        dry_run: false,
    })
    .expect("adopt addon");

    assert_eq!(result.package_id, "plater");
    assert_eq!(result.addons.len(), 1);
    assert_eq!(result.addons[0].directory_name, "Plater");
    assert!(matches!(result.source, AddonSourceRef::LocalArchive { .. }));

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("list addons");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert!(inventory.untracked_addons.is_empty());
    assert_eq!(inventory.tracked_packages[0].package_id, "plater");

    let archive_path = addon_state_paths(&installation)
        .adopted_dir
        .join("plater.zip");
    assert!(archive_path.exists());
}

#[test]
fn adopt_addons_requires_explicit_package_id_for_multi_addon_package() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    for addon_name in ["WeakAuras", "SharedMedia"] {
        let addon_dir = installation.addon_dir.join(addon_name);
        fs::create_dir_all(&addon_dir).expect("addon dir");
        fs::write(
            addon_dir.join(format!("{addon_name}.toc")),
            format!("## Interface: 110000\n## Title: {addon_name}\n"),
        )
        .expect("write toc");
    }

    let error = adopt_addons(AdoptAddonsRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        addon_directories: vec!["WeakAuras".to_string(), "SharedMedia".to_string()],
        package_id: None,
        archive_output_path: None,
        dry_run: false,
    })
    .expect_err("multi-addon adopt without package id should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("package id is required"));
}

#[test]
fn adopt_addons_dry_run_plans_snapshot_without_writing_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let addon_dir = installation.addon_dir.join("Details");
    fs::create_dir_all(&addon_dir).expect("details dir");
    fs::write(
        addon_dir.join("Details.toc"),
        "## Interface: 110000\n## Title: Details!\n",
    )
    .expect("write toc");

    let result = adopt_addons(AdoptAddonsRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        addon_directories: vec!["Details".to_string()],
        package_id: Some("Guild UI Snapshot".to_string()),
        archive_output_path: Some(temp.path().join("exports").join("guild-ui.zip")),
        dry_run: true,
    })
    .expect("dry-run adopt");

    assert!(result.dry_run);
    assert_eq!(result.package_id, "guild-ui-snapshot");
    assert!(!addon_state_paths(&installation).registry_path.exists());
    assert!(!temp.path().join("exports").join("guild-ui.zip").exists());
}
