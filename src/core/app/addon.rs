use crate::core::addon::{
    install_addon_task_with_provider, list_addons, remove_addons_task, search_addons_with_provider,
    update_addons_task_with_provider,
};
use crate::core::app::{
    AddonInventoryResult, AddonSearchCatalogResult, AppRuntime, InstallAddonAppRequest,
    InstalledAddonPackageResult, ListAddonsRequest, RemoveAddonAppRequest,
    RemovedAddonPackageResult, SearchAddonsRequest, UpdateAddonAppRequest,
    UpdatedAddonPackageResult, task_support,
};
use crate::core::error::AppResult;
use crate::core::task::{CancellationToken, TaskProgressEvent, TaskProgressSink, TaskRun};

#[derive(Debug, Clone, Default)]
pub struct AddonService {
    runtime: AppRuntime,
}

impl AddonService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn search(&self, request: SearchAddonsRequest) -> AppResult<AddonSearchCatalogResult> {
        let results = search_addons_with_provider(self.runtime.addon_provider(), request.into())?;
        Ok(AddonSearchCatalogResult::from(results))
    }

    pub fn list(&self, request: ListAddonsRequest) -> AppResult<AddonInventoryResult> {
        let inventory = list_addons(&request.installation)?;
        Ok(AddonInventoryResult::from(inventory))
    }

    pub fn install(
        &self,
        request: InstallAddonAppRequest,
    ) -> AppResult<InstalledAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_task<TCancel, TProgress>(
        &self,
        request: InstallAddonAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<InstalledAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let installed = install_addon_task_with_provider(
            self.runtime.addon_provider(),
            request.into(),
            cancellation,
            progress,
        )?;
        Ok(InstalledAddonPackageResult::from(installed))
    }

    pub fn install_collecting_progress(
        &self,
        request: InstallAddonAppRequest,
    ) -> AppResult<TaskRun<InstalledAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_with_callbacks<FCancel, FProgress>(
        &self,
        request: InstallAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<InstalledAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn update(&self, request: UpdateAddonAppRequest) -> AppResult<UpdatedAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_task<TCancel, TProgress>(
        &self,
        request: UpdateAddonAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<UpdatedAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let updated = update_addons_task_with_provider(
            self.runtime.addon_provider(),
            request.into(),
            cancellation,
            progress,
        )?;
        Ok(UpdatedAddonPackageResult::from(updated))
    }

    pub fn update_collecting_progress(
        &self,
        request: UpdateAddonAppRequest,
    ) -> AppResult<TaskRun<UpdatedAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_with_callbacks<FCancel, FProgress>(
        &self,
        request: UpdateAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<UpdatedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn remove(&self, request: RemoveAddonAppRequest) -> AppResult<RemovedAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }

    pub fn remove_task<TCancel, TProgress>(
        &self,
        request: RemoveAddonAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<RemovedAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let removed = remove_addons_task(request.into(), cancellation, progress)?;
        Ok(RemovedAddonPackageResult::from(removed))
    }

    pub fn remove_collecting_progress(
        &self,
        request: RemoveAddonAppRequest,
    ) -> AppResult<TaskRun<RemovedAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }

    pub fn remove_with_callbacks<FCancel, FProgress>(
        &self,
        request: RemoveAddonAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<RemovedAddonPackageResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;
    use crate::core::addon::{
        AddonProvider, AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult,
        AddonSourceRef, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
        MaterializedAddonSource,
    };
    use crate::core::app::AppRuntime;
    use crate::core::error::AppError;
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
    use crate::core::task::{
        NeverCancel, TaskKind, TaskPhase, TaskProgressEvent, VecTaskProgressSink,
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

        let service =
            AddonService::with_runtime(AppRuntime::with_addon_provider(FakeAddonProvider {
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
        assert!(seen.borrow().iter().any(|event| {
            event.task == TaskKind::AddonUpdate
                && event.phase == TaskPhase::Executing
                && event.message.contains("Writing updated addon directory")
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

    fn create_empty_installation(root: &Path) -> DetectedFlavorInstallation {
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
}
