use std::cell::{Cell, RefCell};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::addon::{
    AddonProvider, AddonSearchProviderCatalog, AddonSearchProviderFailure,
    AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult, AddonSourceRef,
    MaterializeSourceInputRequest, MaterializeSourceRefRequest, MaterializedAddonSource,
};
use crate::core::app::{
    AddonDependencyResolutionCapabilityValue, AddonPackageMetadataValue, AddonPolicyService,
    AddonProviderOptionsValue, AddonProviderRetryPolicyValue, AddonService, AdoptAddonsAppRequest,
    AppRuntime, HttpNoValidatorCachePolicyValue, InstallAddonAppRequest,
    InstalledAddonPackageResult, ListAddonsRequest, RelinkAddonAppRequest, RemoveAddonAppRequest,
    ResolvedInstallationValue, SearchAddonsRequest, SetAddonPolicyAppRequest,
    UpdateAddonAppRequest, UpdatedAddonPackageResult,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{
    NeverCancel, TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent, VecTaskProgressSink,
};

mod catalog;
mod install;
mod registry;
mod tasks;

trait AddonServiceTaskTestExt {
    fn install(&self, request: InstallAddonAppRequest) -> AppResult<InstalledAddonPackageResult>;
    fn update(&self, request: UpdateAddonAppRequest) -> AppResult<UpdatedAddonPackageResult>;
}

impl AddonServiceTaskTestExt for AddonService {
    fn install(&self, request: InstallAddonAppRequest) -> AppResult<InstalledAddonPackageResult> {
        self.install_collecting_progress(request)
            .map(|run| run.result)
    }

    fn update(&self, request: UpdateAddonAppRequest) -> AppResult<UpdatedAddonPackageResult> {
        self.update_collecting_progress(request)
            .map(|run| run.result)
    }
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

#[derive(Clone)]
struct FakeUnsupportedDependencyAddonProvider {
    archive_path: PathBuf,
    update_attempts: Arc<AtomicUsize>,
}

impl AddonProvider for FakeUnsupportedDependencyAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        if request.source != "github:owner/repo#plater.zip" {
            return Err(AppError::Validation(format!(
                "unexpected addon source input: {}",
                request.source
            )));
        }

        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: Some("plater.zip".to_string()),
            },
            archive_path: self.archive_path.clone(),
        })
    }

    fn materialize_source_ref(
        &self,
        request: MaterializeSourceRefRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        self.update_attempts.fetch_add(1, Ordering::SeqCst);
        Err(AppError::Validation(format!(
            "app preflight should have rejected update before materializing `{}`",
            request.source.display_name()
        )))
    }

    fn search_addons(
        &self,
        _request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        Ok(Vec::new())
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
