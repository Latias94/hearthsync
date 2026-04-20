use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::addon::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, InstallAddonRequest, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource, install_addon,
};
use crate::core::app::{
    AddonIndexService, AddonIndexUpdateResult, AppRuntime, InspectAddonIndexRequest,
    InstallAddonIndexAppRequest, ResolvedInstallationValue, UpdateAddonIndexAppRequest,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase, TaskProgressEvent};

#[test]
fn addon_index_service_inspects_index_file() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = { kind = "local_archive", path = "WeakAuras.zip" }
"#,
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let inspection = service
        .inspect(InspectAddonIndexRequest { index_path })
        .expect("inspect addon index");

    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.name, "Fixture Index");
    assert_eq!(inspection.packages[0].id, "weakauras");
}

#[test]
fn addon_index_service_install_collecting_progress_returns_index_task_events() {
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
    let index_path = write_index(temp.path(), &archive_path);

    let service = AddonIndexService::new();
    let run = service
        .install_collecting_progress(InstallAddonIndexAppRequest {
            installation,
            index_path,
            name: "weakauras".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        })
        .expect("install from index with collected progress");

    assert_eq!(run.result.package.id, "weakauras");
    assert_addon_index_task_progress(
        &run.progress,
        TaskKind::AddonIndexInstall,
        "Installing addon directory",
    );
}

#[test]
fn addon_index_service_install_with_runtime_uses_injected_provider() {
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
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/WeakAuras.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let service =
        AddonIndexService::with_runtime(AppRuntime::with_addon_provider(FakeAddonProvider {
            archive_path: archive_path.clone(),
        }));
    let result = service
        .install(InstallAddonIndexAppRequest {
            installation,
            index_path,
            name: "weakauras".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        })
        .expect("install from index through injected provider");

    assert_eq!(result.package.id, "weakauras");
    assert_eq!(result.install.package_id, "weakauras");
    assert_eq!(
        result.install.source.kind,
        crate::core::app::AddonSourceKindResult::HttpArchive
    );
    assert_eq!(
        result.install.source.url.as_deref(),
        Some("https://example.invalid/WeakAuras.zip")
    );
}

#[test]
fn addon_index_service_update_with_callbacks_uses_plain_closures() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let installed_archive_path = temp.path().join("Details-installed.zip");
    let updated_archive_path = temp.path().join("Details-updated.zip");
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
        installation: installation.clone().into_domain(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let seen = std::cell::RefCell::new(Vec::new());
    let cancellation_checks = std::cell::Cell::new(0usize);
    let result = service_update_with_callbacks(
        &AddonIndexService::new(),
        UpdateAddonIndexAppRequest {
            installation,
            index_path,
            name: Some("details".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &seen,
        &cancellation_checks,
    )
    .expect("update from index with callbacks");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(seen.borrow().len() >= 4);
    assert!(seen.borrow().iter().any(|event| {
        event.task == TaskKind::AddonIndexUpdate
            && event.phase == TaskPhase::Executing
            && event.message.contains("Writing updated addon directory")
    }));
    assert!(cancellation_checks.get() >= 3);
}

fn service_update_with_callbacks(
    service: &AddonIndexService,
    request: UpdateAddonIndexAppRequest,
    seen: &std::cell::RefCell<Vec<TaskProgressEvent>>,
    cancellation_checks: &std::cell::Cell<usize>,
) -> AppResult<AddonIndexUpdateResult> {
    service.update_with_callbacks(
        request,
        || {
            let next = cancellation_checks.get() + 1;
            cancellation_checks.set(next);
            false
        },
        |event| seen.borrow_mut().push(event),
    )
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

fn write_index(root: &Path, archive_path: &Path) -> std::path::PathBuf {
    let index_path = root.join("addon-index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "details"
name = "Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\"),
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");
    index_path
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
        match request.source {
            AddonSourceRef::HttpArchive { url }
                if url == "https://example.invalid/WeakAuras.zip" =>
            {
                Ok(MaterializedAddonSource {
                    source_ref: request.source.clone(),
                    archive_path: self.archive_path.clone(),
                })
            }
            other => Err(AppError::Validation(format!(
                "unexpected addon source ref: {}",
                other.display_name()
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

fn assert_addon_index_task_progress(
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
