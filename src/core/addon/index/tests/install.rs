use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::super::{
    AddonIndexInstallRequest, install_addon_from_index, install_addon_from_index_task,
};
use super::{
    addon_state_paths, assert_addon_index_task_progress, create_addon_archive,
    create_fixture_installation, normalized_archive_path, write_index,
};
use crate::core::addon::{AddonSourceRef, list_addons};
use crate::core::task::{NeverCancel, TaskKind, VecTaskProgressSink};

#[test]
fn install_addon_from_index_installs_selected_package() {
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
    let index_path = write_index(temp.path(), &archive_path);

    let result = install_addon_from_index(AddonIndexInstallRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: "details".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
    })
    .expect("install from index");

    assert_eq!(result.package.id, "details");
    assert!(
        installation
            .addon_dir
            .join("Details")
            .join("Details.toc")
            .exists()
    );
}

#[test]
fn install_addon_from_index_task_reports_index_install_progress() {
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
    let index_path = write_index(temp.path(), &archive_path);

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = install_addon_from_index_task(
        AddonIndexInstallRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: "details".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        },
        &cancellation,
        &mut progress,
    )
    .expect("install from index task");

    assert_eq!(result.package.id, "details");
    assert_addon_index_task_progress(
        progress.events(),
        TaskKind::AddonIndexInstall,
        "Installing addon directory",
    );
}

#[test]
fn install_addon_from_index_resolves_relative_local_archive_against_index_path() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_dir = temp.path().join("archives");
    let archive_path = archive_dir.join("details.zip");
    fs::create_dir_all(&archive_dir).expect("archive dir");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = write_index(
        temp.path(),
        Path::new("archives").join("details.zip").as_path(),
    );

    let result = install_addon_from_index(AddonIndexInstallRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: "details".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
    })
    .expect("install from relative index source");

    assert_eq!(result.package.id, "details");
    assert!(
        installation
            .addon_dir
            .join("Details")
            .join("Details.toc")
            .exists()
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::LocalArchive {
            path: normalized_archive_path(&archive_path),
        }
    );
}
