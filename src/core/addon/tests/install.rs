use std::fs;

use tempfile::tempdir;

use super::{
    addon_state_paths, create_addon_archive, create_addon_archive_with_symlink_entry,
    create_fixture_installation, create_fixture_installation_for_platform,
};
use crate::core::addon::{InstallAddonRequest, install_addon, list_addons};
use crate::core::error::AppError;
use crate::core::install::HostPlatform;

#[test]
fn install_addon_from_local_archive_writes_files_and_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("weakauras-pack.zip");

    create_addon_archive(
        &archive_path,
        &[
            (
                "WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Title: WeakAuras\n## Version: 1.0.0\n",
            ),
            ("WeakAuras/Core.lua", "print('wa')"),
            (
                "SharedMedia/SharedMedia.toc",
                "## Interface: 110000\n## Title: SharedMedia\n",
            ),
        ],
    );

    let result = install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    assert_eq!(result.package_id, "weakauras-pack");
    assert_eq!(result.addons.len(), 2);
    assert!(
        installation
            .addon_dir
            .join("WeakAuras")
            .join("Core.lua")
            .exists()
    );
    assert!(addon_state_paths(&installation).registry_path.exists());

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("list addons");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert!(inventory.untracked_addons.is_empty());
}

#[test]
fn install_addon_persists_registry_and_lock_without_temp_sidecars() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details-pack.zip");

    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Title: Details!\n",
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

    let hearthsync_dir = addon_state_paths(&installation).root_dir;
    let mut entries = fs::read_dir(&hearthsync_dir)
        .expect("hearthsync dir")
        .map(|entry| {
            entry
                .expect("dir entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect::<Vec<_>>();
    entries.sort();

    assert_eq!(
        entries,
        vec!["addons.toml".to_string(), "lock.toml".to_string()]
    );
}

#[test]
fn install_addon_from_archive_accepts_variant_toc_names() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("dbm-pack.zip");

    create_addon_archive(
        &archive_path,
        &[
            (
                "DBM-Core/DBM-Core_Mainline.toc",
                "## Interface: 110000\n## Title: DBM Core\n",
            ),
            ("DBM-Core/Core.lua", "print('dbm')"),
        ],
    );

    let result = install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon with variant toc name");

    assert_eq!(result.addons.len(), 1);
    assert_eq!(result.addons[0].directory_name, "DBM-Core");
    assert_eq!(
        result.addons[0].toc_file.as_deref(),
        Some("DBM-Core_Mainline.toc")
    );
    assert!(
        installation
            .addon_dir
            .join("DBM-Core")
            .join("Core.lua")
            .exists()
    );
}

#[test]
fn install_addon_from_local_archive_keeps_case_mixed_subtree_files_on_windows() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation_for_platform(temp.path(), HostPlatform::Windows);
    let archive_path = temp.path().join("mixed-case-pack.zip");

    create_addon_archive(
        &archive_path,
        &[
            (
                "WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Title: WeakAuras\n",
            ),
            ("weakauras/Core.lua", "print('wa core')"),
            ("weakauras/Modules/Module.lua", "print('wa module')"),
        ],
    );

    let result = install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon with case-mixed subtree");

    assert_eq!(result.addons.len(), 1);
    assert_eq!(result.addons[0].directory_name, "WeakAuras");
    assert_eq!(
        fs::read_to_string(installation.addon_dir.join("WeakAuras").join("Core.lua"))
            .expect("core file"),
        "print('wa core')"
    );
    assert_eq!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("Modules")
                .join("Module.lua")
        )
        .expect("module file"),
        "print('wa module')"
    );
}

#[test]
fn install_addon_from_local_archive_rejects_symlink_entries() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("symlink-pack.zip");

    create_addon_archive_with_symlink_entry(
        &archive_path,
        "WeakAuras/WeakAuras.toc",
        "../Shared/WeakAuras.toc",
    );

    let error = install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect_err("symlink archive entry should fail");

    let message = error.to_string();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("WeakAuras/WeakAuras.toc"));
    assert!(!installation.addon_dir.join("WeakAuras").exists());
    assert!(!addon_state_paths(&installation).registry_path.exists());
}

#[test]
fn install_addon_from_local_archive_rejects_case_insensitive_addon_root_collisions_on_windows() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation_for_platform(temp.path(), HostPlatform::Windows);
    let archive_path = temp.path().join("collision-pack.zip");

    create_addon_archive(
        &archive_path,
        &[
            (
                "WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Title: WeakAuras\n",
            ),
            (
                "weakauras/weakauras.toc",
                "## Interface: 110000\n## Title: WeakAuras Lower\n",
            ),
        ],
    );

    let error = install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect_err("case-insensitive addon roots should fail");

    let message = error.to_string();
    assert!(matches!(error, AppError::Validation(_)));
    assert!(message.contains("case-insensitive addon directory collisions"));
    assert!(message.contains("WeakAuras"));
    assert!(message.contains("weakauras"));
    assert!(!installation.addon_dir.join("WeakAuras").exists());
}
