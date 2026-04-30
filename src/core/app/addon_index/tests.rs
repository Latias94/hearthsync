use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::addon::{
    AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
    AddonSourceRef, InstallAddonRequest, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource, install_addon,
};
use crate::core::app::{
    AddonDependencyResolutionCapabilityValue, AddonIndexAttachPackageStatusResult,
    AddonIndexInspectionWarningCodeResult, AddonIndexInspectionWarningSeverityResult,
    AddonIndexPackageSuggestionStatusResult, AddonIndexScaffoldResult, AddonIndexService,
    AddonIndexTrackedMatchStrategyResult, AddonIndexUpdateResult, AddonPolicyService, AddonService,
    AppRuntime, AttachAddonIndexAppRequest, InspectAddonIndexRequest, InstallAddonAppRequest,
    InstallAddonIndexAppRequest, RelinkAddonIndexAppRequest, ResolvedInstallationValue,
    ScaffoldAddonIndexRequest, SetAddonPolicyAppRequest, SuggestAddonIndexRequest,
    UpdateAddonIndexAppRequest,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent};

fn addon_state_paths(
    installation: &crate::core::install::DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::default(),
        installation,
    )
    .expect("addon state paths")
}

mod curation;
mod inspect_validate;
mod operations;
mod provider_runtime;
mod update;

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

#[derive(Clone)]
struct FakeDownloadProgressAddonProvider {
    archive_path: PathBuf,
}

impl AddonProvider for FakeDownloadProgressAddonProvider {
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
        let source_ref = request.source.clone();
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

    fn search_addons(
        &self,
        _request: ProviderAddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        Ok(Vec::new())
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
            "app preflight should have rejected addon-index update before materializing `{}`",
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

#[derive(Clone)]
struct FakeDisplayNamePreflightAddonProvider {
    archive_path: PathBuf,
    update_attempts: Arc<AtomicUsize>,
}

impl AddonProvider for FakeDisplayNamePreflightAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        if request.source != "github:legacy-owner/legacy-repo#plater.zip" {
            return Err(AppError::Validation(format!(
                "unexpected addon source input: {}",
                request.source
            )));
        }

        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::GitHubRelease {
                owner: "legacy-owner".to_string(),
                repo: "legacy-repo".to_string(),
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
            "app preflight should have rejected addon-index update before materializing `{}`",
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

#[derive(Clone)]
struct FakeDeferredDependencyGuidanceAddonProvider {
    archive_path: PathBuf,
    update_attempts: Arc<AtomicUsize>,
}

impl AddonProvider for FakeDeferredDependencyGuidanceAddonProvider {
    fn materialize_source_input(
        &self,
        request: MaterializeSourceInputRequest<'_>,
    ) -> AppResult<MaterializedAddonSource> {
        if request.source != "github:legacy-owner/legacy-repo#plater.zip" {
            return Err(AppError::Validation(format!(
                "unexpected addon source input: {}",
                request.source
            )));
        }

        Ok(MaterializedAddonSource {
            source_ref: AddonSourceRef::GitHubRelease {
                owner: "legacy-owner".to_string(),
                repo: "legacy-repo".to_string(),
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
    assert!(phases.contains(&(task, TaskPhase::Executing)));
    assert!(events.iter().any(|event| {
        event.task == task
            && event.phase == TaskPhase::Executing
            && event.message.contains(executing_detail)
    }));
}
