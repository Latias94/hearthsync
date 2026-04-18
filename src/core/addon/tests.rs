use std::cell::Cell;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::package_prep::prepare_package_from_source_input_with_provider;
use super::provider::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
    MaterializedAddonSource,
};
use super::{
    InstallAddonRequest, RemoveAddonRequest, SearchAddonRequest, UpdateAddonRequest, install_addon,
    install_addon_task, install_addon_task_with_provider, list_addons, remove_addons,
    remove_addons_task, search_addons_with_provider, update_addons, update_addons_task,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::TaskProgressEvent;
use crate::core::task::{CancellationToken, NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

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
    assert!(
        installation
            .addon_dir
            .join(".hearthsync")
            .join("addons.toml")
            .exists()
    );

    let inventory = list_addons(&installation).expect("list addons");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert!(inventory.untracked_addons.is_empty());
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
fn list_addons_reports_untracked_directories() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());

    fs::create_dir_all(installation.addon_dir.join("Plater")).expect("plater dir");
    fs::write(
        installation.addon_dir.join("Plater").join("Plater.toc"),
        "## Interface: 110000",
    )
    .expect("plater toc");

    let inventory = list_addons(&installation).expect("list addons");
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
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    let result = remove_addons(RemoveAddonRequest {
        installation: installation.clone(),
        name: "Plater".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("remove addon");

    assert_eq!(result.removed_addons, vec!["Plater".to_string()]);
    assert!(result.registry_cleaned);
    assert!(!installation.addon_dir.join("Plater").exists());
    assert!(
        !installation
            .addon_dir
            .join(".hearthsync")
            .join("addons.toml")
            .exists()
    );
    assert!(!installation.addon_dir.join(".hearthsync").exists());
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
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install addon");

    let result = remove_addons(RemoveAddonRequest {
        installation: installation.clone(),
        name: "details-pack".to_string(),
        dry_run: true,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("dry-run remove");

    assert_eq!(result.removed_addons, vec!["Details".to_string()]);
    assert!(!result.registry_cleaned);
    assert!(installation.addon_dir.join("Details").exists());
    assert!(
        installation
            .addon_dir
            .join(".hearthsync")
            .join("addons.toml")
            .exists()
    );
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

fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
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
        platform: HostPlatform::Windows,
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
    assert!(
        phases
            .iter()
            .any(|phase| *phase == (task, TaskPhase::Executing))
    );
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
