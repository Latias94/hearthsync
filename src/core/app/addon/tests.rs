use std::cell::{Cell, RefCell};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::addon::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
    MaterializedAddonSource,
};
use crate::core::app::{
    AddonPackageMetadataValue, AddonService, AppRuntime, InstallAddonAppRequest, ListAddonsRequest,
    RemoveAddonAppRequest, ResolvedInstallationValue, SearchAddonsRequest, UpdateAddonAppRequest,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{
    NeverCancel, TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent, VecTaskProgressSink,
};

#[test]
fn addon_service_install_and_list_roundtrip_local_archive() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    let installed = service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");
    let inventory = service
        .list(ListAddonsRequest { installation })
        .expect("list addons");

    assert_eq!(installed.package_id, "weakauras");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert!(inventory.untracked_addons.is_empty());
}

#[test]
fn addon_service_search_returns_app_owned_catalog() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let service =
        AddonService::with_runtime(AppRuntime::with_addon_provider(FakeSearchAddonProvider));

    let results = service
        .search(SearchAddonsRequest {
            installation,
            query: "weak".to_string(),
            limit: 5,
        })
        .expect("search addons");

    assert_eq!(results.query, "weak");
    assert_eq!(results.result_count, 1);
    assert_eq!(results.results[0].provider, "fake-provider");
    assert_eq!(results.results[0].source_label, "curseforge:42");
}

#[test]
fn addon_service_install_and_list_roundtrip_app_owned_metadata() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: Some(AddonPackageMetadataValue {
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

    let inventory = service
        .list(ListAddonsRequest { installation })
        .expect("list addons");
    let metadata = inventory.tracked_packages[0]
        .metadata
        .as_ref()
        .expect("tracked metadata");

    assert_eq!(metadata.index_name.as_deref(), Some("curated"));
    assert_eq!(metadata.index_package_id.as_deref(), Some("details"));
    assert_eq!(metadata.package_name.as_deref(), Some("Details"));
    assert_eq!(metadata.version.as_deref(), Some("1.0.0"));
    assert_eq!(
        metadata.source_url.as_deref(),
        Some("https://example.invalid/details.zip")
    );
    assert_eq!(metadata.supported_flavors, vec!["retail"]);
}

#[test]
fn addon_service_install_collecting_progress_returns_install_task_events() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    let run = service
        .install_collecting_progress(InstallAddonAppRequest {
            installation,
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon with collected progress");

    assert_eq!(run.result.package_id, "weakauras");
    assert!(run.task_id.starts_with("task-"));
    assert!(
        run.progress
            .iter()
            .all(|event| event.task_id.as_deref() == Some(run.task_id.as_str()))
    );
    let step = run
        .progress
        .iter()
        .find(|event| {
            event.task == TaskKind::AddonInstall
                && event.phase == TaskPhase::Executing
                && event.message.contains("Installing addon directory")
        })
        .expect("install step event");
    assert_eq!(step.code, Some(TaskProgressCode::WriteAddonDirectory));
    assert_eq!(step.current, Some(1));
    assert_eq!(step.total, Some(1));
    assert_addon_task_progress(
        &run.progress,
        TaskKind::AddonInstall,
        "Installing addon directory",
    );
}

#[test]
fn addon_service_install_with_runtime_uses_injected_provider() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras-runtime.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::with_runtime(AppRuntime::with_addon_provider(FakeAddonProvider {
        archive_path: archive_path.clone(),
    }));
    let installed = service
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: "https://example.invalid/WeakAuras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon through injected provider");
    let inventory = service
        .list(ListAddonsRequest { installation })
        .expect("list addons");

    assert_eq!(installed.package_id, "weakauras");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(
        inventory.tracked_packages[0].source.kind,
        crate::core::app::AddonSourceKindResult::HttpArchive
    );
    assert_eq!(
        inventory.tracked_packages[0].source.url.as_deref(),
        Some("https://example.invalid/WeakAuras.zip")
    );
}

#[test]
fn addon_service_install_collecting_progress_includes_download_byte_events() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("WeakAuras-progress.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::with_runtime(AppRuntime::with_addon_provider(
        FakeDownloadProgressAddonProvider {
            archive_path: archive_path.clone(),
        },
    ));
    let run = service
        .install_collecting_progress(InstallAddonAppRequest {
            installation,
            source: "https://example.invalid/WeakAuras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon with byte progress");

    let download_events = run
        .progress
        .iter()
        .filter(|event| event.code == Some(TaskProgressCode::DownloadArchive))
        .collect::<Vec<_>>();
    assert_eq!(download_events.len(), 2);
    assert!(
        download_events
            .iter()
            .all(|event| event.phase == TaskPhase::Preparing)
    );
    assert_eq!(download_events[0].bytes_current, Some(0));
    assert_eq!(download_events[0].bytes_total, Some(1024));
    assert_eq!(download_events[1].bytes_current, Some(1024));
    assert_eq!(download_events[1].bytes_total, Some(1024));
    assert_eq!(download_events[1].bytes_per_second, Some(512));
}

#[test]
fn addon_service_update_with_callbacks_uses_plain_closures() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    service
        .install(InstallAddonAppRequest {
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

    let seen = RefCell::new(Vec::new());
    let cancellation_checks = Cell::new(0usize);
    let result = service
        .update_with_callbacks(
            UpdateAddonAppRequest {
                installation,
                name: Some("Details".to_string()),
                dry_run: false,
                backup_output_path: Some(temp.path().join("backups")),
            },
            || {
                let next = cancellation_checks.get() + 1;
                cancellation_checks.set(next);
                false
            },
            |event| seen.borrow_mut().push(event),
        )
        .expect("update addon with callbacks");

    assert_eq!(result.updated_packages.len(), 1);
    assert!(seen.borrow().len() >= 4);
    let callback_task_id = seen.borrow()[0].task_id.clone();
    assert!(
        callback_task_id
            .as_deref()
            .is_some_and(|task_id| task_id.starts_with("task-"))
    );
    assert!(
        seen.borrow()
            .iter()
            .all(|event| event.task_id == callback_task_id)
    );
    assert!(seen.borrow().iter().any(|event| {
        event.task == TaskKind::AddonUpdate
            && event.phase == TaskPhase::Executing
            && event.message.contains("Writing updated addon directory")
            && event.code == Some(TaskProgressCode::WriteAddonDirectory)
            && event.current == Some(1)
            && event.total == Some(1)
    }));
    assert!(cancellation_checks.get() >= 3);
}

#[test]
fn addon_service_remove_task_returns_remove_task_events() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    let service = AddonService::new();
    service
        .install(InstallAddonAppRequest {
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
    let result = service
        .remove_task(
            RemoveAddonAppRequest {
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

fn create_empty_installation(root: &Path) -> ResolvedInstallationValue {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon dir");
    fs::create_dir_all(&wtf_dir).expect("wtf dir");
    fs::create_dir_all(&fonts_dir).expect("fonts dir");

    ResolvedInstallationValue::from_domain(crate::core::install::DetectedFlavorInstallation {
        platform: HostPlatform::Windows,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    })
}

fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(path).expect("archive file");
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

#[derive(Clone)]
struct FakeAddonProvider {
    archive_path: PathBuf,
}

impl AddonProvider for FakeAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        if request.source != "https://example.invalid/WeakAuras.zip" {
            return Err(AppError::Validation(format!(
                "unexpected addon source input: {}",
                request.source
            )));
        }

        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::HttpArchive {
                url: request.source.to_string(),
            },
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Ok(MaterializedAddonSource {
            source_ref: request.source.clone(),
            archive_path: self.archive_path.clone(),
        })
    }

    fn search_addons(
        &self,
        _request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
struct FakeDownloadProgressAddonProvider {
    archive_path: PathBuf,
}

impl AddonProvider for FakeDownloadProgressAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        let source_ref = AddonSourceRef::HttpArchive {
            url: request.source.to_string(),
        };
        request.context.report_download_progress(
            &source_ref,
            "WeakAuras-progress.zip",
            0,
            Some(1024),
            None,
        );
        request.context.report_download_progress(
            &source_ref,
            "WeakAuras-progress.zip",
            1024,
            Some(1024),
            Some(512),
        );

        Ok(MaterializedAddonSource {
            source_ref,
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        _request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Err(AppError::Validation(
            "byte-progress test provider expects source input".to_string(),
        ))
    }

    fn search_addons(
        &self,
        _request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        Ok(Vec::new())
    }
}

#[derive(Clone)]
struct FakeSearchAddonProvider;

impl AddonProvider for FakeSearchAddonProvider {
    fn materialize_source_input(
        &self,
        _request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Err(AppError::Validation(
            "search-only provider does not materialize sources".to_string(),
        ))
    }

    fn materialize_source_ref(
        &self,
        _request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        Err(AppError::Validation(
            "search-only provider does not materialize sources".to_string(),
        ))
    }

    fn search_addons(
        &self,
        request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        assert_eq!(request.query, "weak");
        assert_eq!(request.limit, 5);
        Ok(vec![AddonSearchResult {
            provider: "fake-provider",
            name: "WeakAuras".to_string(),
            summary: Some("fixture search result".to_string()),
            source: AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: None,
            },
            install_hint: "curseforge:42".to_string(),
            website_url: Some("https://example.invalid/weakauras".to_string()),
            provider_project_id: Some(42),
            provider_file_id: None,
            download_count: 100,
        }])
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
