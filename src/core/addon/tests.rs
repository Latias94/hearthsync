use std::cell::{Cell, RefCell};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::package_prep::prepare_package_from_source_input_with_provider;
use super::policy::{AddonReleaseChannel, SetAddonPolicyRequest, set_addon_policy};
use super::provider::{
    AddonDependencyResolutionCapability, AddonProvider,
    AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult, AddonSourceRef,
    AddonSourceResolutionPolicy, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
    MaterializedAddonSource, ResolveAddonDependenciesRequest, ResolvedAddonDependencies,
};
use super::{
    AddonRegistry, AdoptAddonsRequest, InstallAddonRequest, RelinkAddonRequest, RemoveAddonRequest,
    SearchAddonRequest, TrackedAddon, TrackedAddonPackage, UpdateAddonRequest, adopt_addons,
    canonicalize_local_archive_path, install_addon, install_addon_task,
    install_addon_task_with_provider, list_addons, load_registry, relink_addon, remove_addons,
    remove_addons_task, save_registry, search_addons_with_provider, update_addons,
    update_addons_task, update_addons_task_with_provider,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::TaskProgressEvent;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, VecTaskProgressSink,
};

fn addon_state_paths(installation: &DetectedFlavorInstallation) -> super::AddonStatePaths {
    super::AddonStatePaths::for_installation(super::AddonStateStorageKind::default(), installation)
        .expect("addon state paths")
}

fn sidecar_addon_state_paths(installation: &DetectedFlavorInstallation) -> super::AddonStatePaths {
    super::AddonStatePaths::for_installation(super::AddonStateStorageKind::Sidecar, installation)
        .expect("sidecar addon state paths")
}

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
fn load_registry_rejects_case_insensitive_duplicate_package_ids() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let state_paths = sidecar_addon_state_paths(&installation);
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![
            tracked_package("Details", "Details"),
            tracked_package("details", "Omen"),
        ],
    };
    fs::create_dir_all(state_paths.registry_path.parent().expect("registry parent"))
        .expect("registry parent dir");
    fs::write(
        &state_paths.registry_path,
        toml::to_string_pretty(&registry).expect("registry toml"),
    )
    .expect("write invalid registry");

    let error = load_registry(&installation, &state_paths)
        .expect_err("case-insensitive duplicate package ids should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("duplicate tracked addon package id"));
    assert!(message.contains("Details"));
    assert!(message.contains("details"));
}

#[test]
fn save_registry_rejects_case_insensitive_duplicate_addon_directory_owners() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let state_paths = sidecar_addon_state_paths(&installation);
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![
            tracked_package("details", "Details"),
            tracked_package("details-alt", "details"),
        ],
    };

    let error = save_registry(&installation, &state_paths, &registry)
        .expect_err("case-insensitive duplicate addon directory owners should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon directory `details`"));
    assert!(message.contains("tracked package `details-alt`"));
    assert!(message.contains("tracked package `details`"));
    assert!(!state_paths.registry_path.exists());
}

#[test]
fn save_registry_rejects_relative_local_archive_sources() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let state_paths = sidecar_addon_state_paths(&installation);
    let mut package = tracked_package("details", "Details");
    package.source = AddonSourceRef::LocalArchive {
        path: PathBuf::from("archives/details.zip"),
    };
    let registry = AddonRegistry {
        schema_version: 1,
        packages: vec![package],
    };

    let error = save_registry(&installation, &state_paths, &registry)
        .expect_err("relative local archive source should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("tracked addon package `details`"));
    assert!(message.contains("must be absolute"));
    assert!(!state_paths.registry_path.exists());
}

#[test]
fn install_addon_dry_run_rejects_case_insensitive_existing_addon_on_macos_target() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation_for_platform(temp.path(), HostPlatform::MacOs);
    let existing_dir = installation.addon_dir.join("Details");
    fs::create_dir_all(&existing_dir).expect("existing details dir");
    fs::write(
        existing_dir.join("Details.toc"),
        "## Interface: 110000\n## Title: Details!\n",
    )
    .expect("existing toc");
    let archive_path = temp.path().join("details-lower.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "details/details.toc",
            "## Interface: 110000\n## Title: Details Lower\n",
        )],
    );

    let error = install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation,
        source: archive_path.display().to_string(),
        dry_run: true,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect_err("case-insensitive existing addon should fail during planning");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon directories already exist: Details"));
    assert!(!temp.path().join("backups").exists());
}

#[test]
fn install_addon_replace_existing_removes_case_insensitive_existing_addon_on_macos_target() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation_for_platform(temp.path(), HostPlatform::MacOs);
    let existing_dir = installation.addon_dir.join("Details");
    fs::create_dir_all(&existing_dir).expect("existing details dir");
    fs::write(
        existing_dir.join("Details.toc"),
        "## Interface: 110000\n## Title: Details!\n",
    )
    .expect("existing toc");
    fs::write(existing_dir.join("Old.lua"), "print('old')").expect("old file");
    let archive_path = temp.path().join("details-lower.zip");
    create_addon_archive(
        &archive_path,
        &[
            (
                "details/details.toc",
                "## Interface: 110000\n## Title: Details Lower\n",
            ),
            ("details/New.lua", "print('new')"),
        ],
    );

    let result = install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: true,
        metadata: None,
    })
    .expect("replace existing addon");

    assert_eq!(result.replaced_addons, vec!["Details".to_string()]);
    assert_eq!(result.addons[0].directory_name, "details");
    assert!(
        !installation
            .addon_dir
            .join("Details")
            .join("Old.lua")
            .exists()
    );
    assert!(
        installation
            .addon_dir
            .join("details")
            .join("New.lua")
            .exists()
    );
}

#[test]
fn install_addon_task_reports_install_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("weakauras-pack.zip");

    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = install_addon_task(
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &cancellation,
        &mut progress,
    )
    .expect("install addon task");

    assert_eq!(result.package_id, "weakauras-pack");
    assert_addon_task_progress(
        progress.events(),
        TaskKind::AddonInstall,
        "Installing addon directory",
    );
}

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
fn update_addons_applies_curseforge_file_pin_policy() {
    #[derive(Default)]
    struct FakeProvider {
        update_sources: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            self.update_sources
                .borrow_mut()
                .push(request.source.clone());
            let archive_path = request.stage_root.join("curse-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: Some(777),
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set file pin");

    let result = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update addon with file pin");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(
        provider.update_sources.borrow().as_slice(),
        &[AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: Some(777),
        }]
    );
    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: Some(777),
        }
    );
    assert_eq!(inventory.tracked_packages[0].package_id, "curseforge-42");
}

#[test]
fn update_addons_applies_github_tag_pin_policy() {
    #[derive(Default)]
    struct FakeProvider {
        update_sources: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            self.update_sources
                .borrow_mut()
                .push(request.source.clone());
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: None,
        pinned_version: Some("v2.5.0".to_string()),
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set tag pin");

    let result = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update addon with tag pin");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(
        provider.update_sources.borrow().as_slice(),
        &[AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: Some("v2.5.0".to_string()),
            asset_name: Some("plater.zip".to_string()),
        }]
    );
    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::GitHubRelease {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            tag: Some("v2.5.0".to_string()),
            asset_name: Some("plater.zip".to_string()),
        }
    );
    assert_eq!(inventory.tracked_packages[0].package_id, "plater");
}

#[test]
fn update_addons_forwards_resolution_policy_into_provider_context() {
    #[derive(Default)]
    struct FakeProvider {
        update_requests: RefCell<Vec<(AddonSourceRef, AddonSourceResolutionPolicy)>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            self.update_requests
                .borrow_mut()
                .push((request.source.clone(), request.context.resolution_policy()));
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: Some(AddonReleaseChannel::Beta),
        allow_prerelease: Some(true),
        install_dependencies: None,
    })
    .expect("set resolution policy");

    let result = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update addon with resolution policy");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(
        provider.update_requests.borrow().as_slice(),
        &[(
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: Some("plater.zip".to_string()),
            },
            AddonSourceResolutionPolicy {
                release_channel: Some(AddonReleaseChannel::Beta),
                allow_prerelease: Some(true),
            },
        )]
    );
}

#[test]
fn update_addons_installs_missing_required_dependencies_when_policy_enabled() {
    #[derive(Default)]
    struct FakeProvider {
        dependency_requests: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    request.stage_root.join("curse-addon-update.zip")
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    request.stage_root.join("sharedmedia-addon.zip")
                }
                source => {
                    return Err(AppError::Validation(format!(
                        "unexpected source during dependency install test: {}",
                        source.display_name()
                    )));
                }
            };

            let entries = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => vec![(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => vec![(
                    "SharedMedia/SharedMedia.toc",
                    "## Interface: 120000\n## Version: 1.0.0\n",
                )],
                _ => unreachable!(),
            };
            create_addon_archive(&archive_path, &entries);

            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn dependency_resolution_capability(
            &self,
            source: &AddonSourceRef,
        ) -> AddonDependencyResolutionCapability {
            match source {
                AddonSourceRef::CurseForgeMod { .. } => {
                    AddonDependencyResolutionCapability::missing_required_only()
                }
                _ => AddonDependencyResolutionCapability::Unsupported,
            }
        }

        fn resolve_addon_dependencies(
            &self,
            request: ResolveAddonDependenciesRequest<'_>,
        ) -> AppResult<ResolvedAddonDependencies> {
            self.dependency_requests
                .borrow_mut()
                .push(request.source.clone());
            match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(vec![
                        AddonSourceRef::CurseForgeMod {
                            mod_id: 99,
                            file_id: None,
                        },
                    ]))
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(Vec::new()))
                }
                source => Err(AppError::Validation(format!(
                    "unexpected source during dependency resolution test: {}",
                    source.display_name()
                ))),
            }
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let result = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update addon with dependency installation");

    assert_eq!(result.updated_packages.len(), 1);
    assert_eq!(result.installed_dependency_packages.len(), 1);
    assert_eq!(
        result.installed_dependency_packages[0].package_id,
        "curseforge-99"
    );
    assert_eq!(
        provider.dependency_requests.borrow().as_slice(),
        &[
            AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: None,
            },
            AddonSourceRef::CurseForgeMod {
                mod_id: 99,
                file_id: None,
            },
        ]
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 2);
    assert!(
        inventory
            .tracked_packages
            .iter()
            .any(|package| package.package_id == "curseforge-42")
    );
    assert!(
        inventory
            .tracked_packages
            .iter()
            .any(|package| package.package_id == "curseforge-99")
    );
}

#[test]
fn update_addons_rolls_back_when_dependency_install_fails_after_primary_update() {
    #[derive(Default)]
    struct FakeProvider {
        dependency_requests: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    request.stage_root.join("curse-addon-update.zip")
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    request.stage_root.join("sharedmedia-addon.zip")
                }
                source => {
                    return Err(AppError::Validation(format!(
                        "unexpected source during dependency rollback test: {}",
                        source.display_name()
                    )));
                }
            };

            let entries = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => vec![(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => vec![(
                    "SharedMedia/SharedMedia.toc",
                    "## Interface: 120000\n## Version: 1.0.0\n",
                )],
                _ => unreachable!(),
            };
            create_addon_archive(&archive_path, &entries);

            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn dependency_resolution_capability(
            &self,
            source: &AddonSourceRef,
        ) -> AddonDependencyResolutionCapability {
            match source {
                AddonSourceRef::CurseForgeMod { .. } => {
                    AddonDependencyResolutionCapability::missing_required_only()
                }
                _ => AddonDependencyResolutionCapability::Unsupported,
            }
        }

        fn resolve_addon_dependencies(
            &self,
            request: ResolveAddonDependenciesRequest<'_>,
        ) -> AppResult<ResolvedAddonDependencies> {
            self.dependency_requests
                .borrow_mut()
                .push(request.source.clone());
            match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(vec![
                        AddonSourceRef::CurseForgeMod {
                            mod_id: 99,
                            file_id: None,
                        },
                    ]))
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(Vec::new()))
                }
                source => Err(AppError::Validation(format!(
                    "unexpected source during dependency rollback resolution test: {}",
                    source.display_name()
                ))),
            }
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    let local_dependency_dir = installation.addon_dir.join("SharedMedia");
    fs::create_dir_all(&local_dependency_dir).expect("create local dependency conflict");
    fs::write(
        local_dependency_dir.join("SharedMedia.toc"),
        "## Interface: 110000\n## Version: local\n",
    )
    .expect("write local dependency conflict");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let error = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("dependency install conflict should roll back");

    let message = error.to_string();
    assert!(message.contains("rollback restored"));
    assert!(message.contains("addon directory already exists"));
    assert!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("weakauras toc after rollback")
        .contains("1.0.0")
    );
    assert!(
        fs::read_to_string(local_dependency_dir.join("SharedMedia.toc"))
            .expect("local dependency after rollback")
            .contains("local")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(inventory.tracked_packages[0].package_id, "curseforge-42");
}

#[test]
fn update_addons_rejects_dependency_installation_for_unsupported_sources() {
    #[derive(Default)]
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider;
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install provider-backed addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let error = update_addons_task_with_provider(
        &provider,
        UpdateAddonRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("unsupported dependency installation should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
}

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
fn search_addons_can_use_fake_provider() {
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            _request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            panic!("materialize_source_input should not be called in this test")
        }

        fn materialize_source_ref(
            &self,
            _request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            panic!("materialize_source_ref should not be called in this test")
        }

        fn search_addons(
            &self,
            request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            assert_eq!(request.query, "WeakAuras");
            assert_eq!(request.flavor, WowFlavor::Retail);
            assert_eq!(request.limit, 5);
            Ok(vec![AddonSearchResult {
                provider: "fake",
                name: "WeakAuras".to_string(),
                summary: Some("fixture result".to_string()),
                source: AddonSourceRef::HttpArchive {
                    url: "https://example.invalid/weakauras.zip".to_string(),
                },
                install_hint: "https://example.invalid/weakauras.zip".to_string(),
                website_url: Some("https://example.invalid/weakauras".to_string()),
                provider_project_id: Some(42),
                provider_file_id: Some(84),
                download_count: 7,
            }])
        }
    }

    let installation = create_fixture_installation(tempdir().expect("temp dir").path());
    let catalog = search_addons_with_provider(
        &FakeProvider,
        SearchAddonRequest {
            installation,
            query: "WeakAuras".to_string(),
            limit: 5,
        },
    )
    .expect("search through fake provider");

    assert_eq!(catalog.query, "WeakAuras");
    assert_eq!(catalog.results.len(), 1);
    assert_eq!(catalog.results[0].provider, "fake");
}

#[test]
fn prepare_package_from_source_input_can_use_fake_provider() {
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            assert_eq!(request.source, "fake:bundle");
            assert_eq!(request.context.target_flavor, Some(WowFlavor::Retail));
            assert!(request.context.cancellation.is_some());
            let archive_path = request.stage_root.join("fake-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::HttpArchive {
                    url: "https://example.invalid/fake-addon.zip".to_string(),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            _request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            panic!("materialize_source_ref should not be called in this test")
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            panic!("search_addons should not be called in this test")
        }
    }

    let cancellation = NeverCancel;
    let prepared = prepare_package_from_source_input_with_provider(
        &FakeProvider,
        "fake:bundle",
        Some(WowFlavor::Retail),
        HostPlatform::Windows,
        &cancellation,
    )
    .expect("prepare package");

    assert_eq!(prepared.package_id, "fake-addon");
    assert_eq!(prepared.addons.len(), 1);
    assert_eq!(prepared.addons[0].addon.directory_name, "WeakAuras");
    assert_eq!(
        prepared.source,
        AddonSourceRef::HttpArchive {
            url: "https://example.invalid/fake-addon.zip".to_string(),
        }
    );
}

#[test]
fn install_addon_task_forwards_cancellation_into_provider_prepare() {
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            assert_eq!(request.context.target_flavor, Some(WowFlavor::Retail));
            assert!(
                request
                    .context
                    .cancellation
                    .expect("provider cancellation token")
                    .is_cancelled()
            );
            Err(AppError::Cancelled(
                "addon provider download cancelled".to_string(),
            ))
        }

        fn materialize_source_ref(
            &self,
            _request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            panic!("materialize_source_ref should not be called in this test")
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            panic!("search_addons should not be called in this test")
        }
    }

    struct FlipOnSecondCheck {
        checks: Cell<usize>,
    }

    impl CancellationToken for FlipOnSecondCheck {
        fn is_cancelled(&self) -> bool {
            let next = self.checks.get() + 1;
            self.checks.set(next);
            next >= 2
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let cancellation = FlipOnSecondCheck {
        checks: Cell::new(0),
    };
    let mut progress = VecTaskProgressSink::default();

    let error = install_addon_task_with_provider(
        &FakeProvider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            source: "fake:bundle".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("provider cancellation should bubble out");

    assert!(matches!(error, AppError::Cancelled(_)));
    assert_eq!(cancellation.checks.get(), 2);
    assert_eq!(
        progress
            .events()
            .iter()
            .map(|event| (event.task, event.phase))
            .collect::<Vec<_>>(),
        vec![(TaskKind::AddonInstall, TaskPhase::Preparing)]
    );
}

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

#[test]
fn relink_addon_updates_tracked_source_and_clears_metadata_without_reinstalling_files() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive = temp.path().join("Details.zip");
    let relink_archive = temp.path().join("Details-release.zip");
    create_addon_archive(
        &installed_archive,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &relink_archive,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: Some(super::AddonPackageMetadata {
            index_name: Some("curated".to_string()),
            index_package_id: Some("details".to_string()),
            package_name: Some("Details".to_string()),
            version: Some("1.0.0".to_string()),
            source_url: Some("https://example.invalid/details.zip".to_string()),
            website_url: Some("https://example.invalid/details".to_string()),
            source_sha256: Some("abc123".to_string()),
            supported_flavors: vec!["retail".to_string()],
        }),
    })
    .expect("install addon");

    let result = relink_addon(RelinkAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        name: "Details".to_string(),
        source: relink_archive.display().to_string(),
        dry_run: false,
    })
    .expect("relink addon");

    assert_eq!(result.package_id, "details");
    assert!(result.cleared_metadata);

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("list addons");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::LocalArchive {
            path: canonicalize_local_archive_path(&relink_archive)
                .expect("normalized relink archive"),
        }
    );
    assert!(inventory.tracked_packages[0].metadata.is_none());
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("installed toc")
            .contains("1.0.0")
    );
}

#[test]
fn relink_addon_rejects_incompatible_addon_directory_sets() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive = temp.path().join("Plater.zip");
    let incompatible_archive = temp.path().join("Plater-remote.zip");
    create_addon_archive(
        &installed_archive,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &incompatible_archive,
        &[(
            "PlaterOptions/PlaterOptions.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    let error = relink_addon(RelinkAddonRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        name: "Plater".to_string(),
        source: incompatible_archive.display().to_string(),
        dry_run: false,
    })
    .expect_err("incompatible relink should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon directory sets must match exactly"));
    assert!(message.contains("missing from source: Plater"));
    assert!(message.contains("extra from source: PlaterOptions"));
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

fn tracked_package(package_id: &str, addon_directory: &str) -> TrackedAddonPackage {
    TrackedAddonPackage {
        package_id: package_id.to_string(),
        source: AddonSourceRef::HttpArchive {
            url: format!("https://example.invalid/{package_id}.zip"),
        },
        installed_at: "2026-04-28T00:00:00Z".to_string(),
        updated_at: "2026-04-28T00:00:00Z".to_string(),
        addons: vec![TrackedAddon {
            directory_name: addon_directory.to_string(),
            toc_file: Some(format!("{addon_directory}.toc")),
            title: Some(addon_directory.to_string()),
            version: Some("1.0.0".to_string()),
        }],
        metadata: None,
    }
}

fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
    create_fixture_installation_for_platform(root, HostPlatform::Windows)
}

fn create_fixture_installation_for_platform(
    root: &Path,
    platform: HostPlatform,
) -> DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    DetectedFlavorInstallation {
        platform,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn assert_addon_task_progress(
    events: &[TaskProgressEvent],
    task: TaskKind,
    executing_detail: &str,
) {
    let phases = events
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();

    assert_eq!(phases.first(), Some(&(task, TaskPhase::Preparing)));
    assert_eq!(phases.last(), Some(&(task, TaskPhase::Completed)));
    assert!(phases.contains(&(task, TaskPhase::BackingUp)));
    assert!(phases.contains(&(task, TaskPhase::Executing)));
    assert!(events.iter().any(|event| {
        event.task == task
            && event.phase == TaskPhase::Executing
            && event.message.contains(executing_detail)
    }));
}

fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
    let file = File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(
            name.replace('\\', "/"),
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start file");
        zip.write_all(content.as_bytes()).expect("write file");
    }
    zip.finish().expect("finish zip");
}

fn create_addon_archive_with_symlink_entry(path: &Path, name: &str, target: &str) {
    let file = File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    zip.add_symlink(name, target, SimpleFileOptions::default())
        .expect("add symlink entry");
    zip.finish().expect("finish zip");
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
