use std::fs;

use tempfile::tempdir;

use super::{
    addon_state_paths, assert_addon_task_progress, create_addon_archive,
    create_fixture_installation,
};
use crate::core::addon::{
    InstallAddonRequest, RemoveAddonRequest, install_addon, list_addons, remove_addons,
    remove_addons_task,
};
use crate::core::error::AppError;
use crate::core::task::{NeverCancel, TaskKind, VecTaskProgressSink};

#[test]
fn list_addons_reports_untracked_directories() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());

    fs::create_dir_all(installation.addon_dir.join("Plater")).expect("plater dir");
    fs::write(
        installation.addon_dir.join("Plater").join("Plater.toc"),
        "## Interface: 110000",
    )
    .expect("plater toc");

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("list addons");
    assert!(inventory.tracked_packages.is_empty());
    assert_eq!(inventory.untracked_addons, vec!["Plater".to_string()]);
}

#[test]
fn remove_addons_removes_directories_and_cleans_registry_when_empty() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("plater-pack.zip");

    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
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

    let result = remove_addons(RemoveAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: "Plater".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("remove addon");

    assert_eq!(result.removed_addons, vec!["Plater".to_string()]);
    assert!(result.registry_cleaned);
    assert!(!installation.addon_dir.join("Plater").exists());
    assert!(!addon_state_paths(&installation).registry_path.exists());
    assert!(!addon_state_paths(&installation).root_dir.exists());
}

#[test]
fn remove_addons_task_reports_remove_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("plater-pack.zip");

    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
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

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = remove_addons_task(
        RemoveAddonRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            name: "Plater".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &cancellation,
        &mut progress,
    )
    .expect("remove addon task");

    assert_eq!(result.removed_addons, vec!["Plater".to_string()]);
    assert_addon_task_progress(
        progress.events(),
        TaskKind::AddonRemove,
        "Removing addon directory",
    );
}

#[test]
fn remove_addons_dry_run_keeps_files_and_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details-pack.zip");

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

    let result = remove_addons(RemoveAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: "details-pack".to_string(),
        dry_run: true,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("dry-run remove");

    assert_eq!(result.removed_addons, vec!["Details".to_string()]);
    assert!(!result.registry_cleaned);
    assert!(installation.addon_dir.join("Details").exists());
    assert!(addon_state_paths(&installation).registry_path.exists());
}

#[test]
fn remove_addons_without_tracked_registry_reports_generic_bootstrap_guidance_when_empty() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());

    let error = remove_addons(RemoveAddonRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        name: "Plater".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect_err("missing tracked registry should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon install"));
    assert!(message.contains("addon index install"));
    assert!(message.contains("addon adopt"));
}
