use std::fs;

use tempfile::tempdir;

use super::{
    addon_state_paths, assert_addon_task_progress, create_addon_archive,
    create_fixture_installation,
};
use crate::core::addon::policy::{SetAddonPolicyRequest, set_addon_policy};
use crate::core::addon::{
    InstallAddonRequest, UpdateAddonRequest, install_addon, update_addons, update_addons_task,
};
use crate::core::error::AppError;
use crate::core::task::{NeverCancel, TaskKind, VecTaskProgressSink};

#[test]
fn update_addons_reuses_recorded_source() {
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

    create_addon_archive(
        &archive_path,
        &[
            (
                "Details/Details.toc",
                "## Interface: 120000\n## Version: 2.0.0\n",
            ),
            ("Details/Core.lua", "print('updated')"),
        ],
    );

    let result = update_addons(UpdateAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: Some("Details".to_string()),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update addons");

    assert_eq!(result.updated_packages.len(), 1);
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
    assert!(
        installation
            .addon_dir
            .join("Details")
            .join("Core.lua")
            .exists()
    );
}

#[test]
fn update_addons_task_reports_update_progress() {
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

    create_addon_archive(
        &archive_path,
        &[
            (
                "Details/Details.toc",
                "## Interface: 120000\n## Version: 2.0.0\n",
            ),
            ("Details/Core.lua", "print('updated')"),
        ],
    );

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = update_addons_task(
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            name: Some("Details".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &cancellation,
        &mut progress,
    )
    .expect("update addon task");

    assert_eq!(result.updated_packages.len(), 1);
    assert_addon_task_progress(
        progress.events(),
        TaskKind::AddonUpdate,
        "Writing updated addon directory",
    );
}

#[test]
fn update_addons_skips_ignored_packages_in_bulk_runs() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let details_archive_path = temp.path().join("details-pack.zip");
    let omen_archive_path = temp.path().join("omen-pack.zip");

    create_addon_archive(
        &details_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &omen_archive_path,
        &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: details_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: omen_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install omen");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "details-pack".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set ignored policy");

    create_addon_archive(
        &details_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    create_addon_archive(
        &omen_archive_path,
        &[("Omen/Omen.toc", "## Interface: 120000\n## Version: 2.0.0\n")],
    );

    let result = update_addons(UpdateAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("bulk-backups")),
    })
    .expect("update addons");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(result.updated_packages[0].package_id, "omen-pack");
    assert_eq!(result.ignored_packages, vec!["details-pack".to_string()]);
    assert!(result.backup_path.is_some());
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("1.0.0")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Omen").join("Omen.toc"))
            .expect("omen toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_returns_noop_without_backup_when_all_selected_packages_are_ignored() {
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

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "details-pack".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set ignored policy");

    let result = update_addons(UpdateAddonRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("bulk-backups")),
    })
    .expect("no-op update");

    assert!(result.updated_packages.is_empty());
    assert_eq!(result.ignored_packages, vec!["details-pack".to_string()]);
    assert!(result.backup_path.is_none());
    assert!(!temp.path().join("bulk-backups").exists());
}

#[test]
fn update_addons_explicit_name_overrides_ignored_policy() {
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

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "details-pack".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set ignored policy");

    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );

    let result = update_addons(UpdateAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: Some("details".to_string()),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update ignored addon explicitly");

    assert_eq!(result.updated_packages.len(), 1);
    assert!(result.ignored_packages.is_empty());
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_without_tracked_registry_prefers_adopt_guidance_when_local_addons_exist() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let addon_dir = installation.addon_dir.join("Plater");
    fs::create_dir_all(&addon_dir).expect("plater dir");
    fs::write(
        addon_dir.join("Plater.toc"),
        "## Interface: 110000\n## Title: Plater\n",
    )
    .expect("write toc");

    let error = update_addons(UpdateAddonRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect_err("missing tracked registry should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon adopt"));
    assert!(message.contains("existing local addons"));
}
