use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::super::{update_addons_from_index, update_addons_from_index_task};
use super::{
    addon_state_paths, assert_addon_index_task_progress, create_addon_archive,
    create_fixture_installation, normalized_archive_path, write_index,
};
use crate::core::addon::index::AddonIndexUpdateRequest;
use crate::core::addon::{
    AddonSourceRef, InstallAddonRequest, install_addon, list_addons,
    policy::{SetAddonPolicyRequest, set_addon_policy},
};
use crate::core::task::{NeverCancel, TaskKind, VecTaskProgressSink};

#[test]
fn update_addons_from_index_uses_index_source_and_skips_unselected_packages() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = temp.path().join("details-updated.zip");
    let extra_archive_path = temp.path().join("omen.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    create_addon_archive(
        &extra_archive_path,
        &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: extra_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install omen");

    let result = update_addons_from_index(AddonIndexUpdateRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update from index");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Omen").join("Omen.toc"))
            .expect("omen toc")
            .contains("1.0.0")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    let details_package = inventory
        .tracked_packages
        .iter()
        .find(|package| {
            package
                .addons
                .iter()
                .any(|addon| addon.directory_name == "Details")
        })
        .expect("details package");
    assert_eq!(
        details_package.source,
        AddonSourceRef::LocalArchive {
            path: normalized_archive_path(&updated_archive_path),
        }
    );
}

#[test]
fn update_addons_from_index_resolves_relative_local_archive_against_index_path() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_dir = temp.path().join("archives");
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = archive_dir.join("details-updated.zip");
    fs::create_dir_all(&archive_dir).expect("archive dir");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(
        temp.path(),
        Path::new("archives").join("details-updated.zip").as_path(),
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let result = update_addons_from_index(AddonIndexUpdateRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: Some("details".to_string()),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update from relative index source");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("2.0.0")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    let details_package = inventory
        .tracked_packages
        .iter()
        .find(|package| {
            package
                .addons
                .iter()
                .any(|addon| addon.directory_name == "Details")
        })
        .expect("details package");
    assert_eq!(
        details_package.source,
        AddonSourceRef::LocalArchive {
            path: normalized_archive_path(&updated_archive_path),
        }
    );
}

#[test]
fn update_addons_from_index_task_reports_index_update_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = temp.path().join("details-updated.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = update_addons_from_index_task(
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: Some("details".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &cancellation,
        &mut progress,
    )
    .expect("update from index task");

    assert_eq!(result.selected_packages.len(), 1);
    assert_addon_index_task_progress(
        progress.events(),
        TaskKind::AddonIndexUpdate,
        "Writing updated addon directory",
    );
}

#[test]
fn update_addons_from_index_skips_ignored_tracked_packages_in_bulk_runs() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = temp.path().join("details-updated.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "details-installed".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set ignored policy");

    let result = update_addons_from_index(AddonIndexUpdateRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("bulk-backups")),
    })
    .expect("update from index");

    assert!(result.selected_packages.is_empty());
    assert!(result.update.updated_packages.is_empty());
    assert_eq!(
        result.update.ignored_packages,
        vec!["details-installed".to_string()]
    );
    assert!(result.update.backup_path.is_none());
    assert!(!temp.path().join("bulk-backups").exists());
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("1.0.0")
    );
}
