use crate::core::addon::{
    AddonInventory, AddonSearchCatalog, InstallAddonRequest, InstalledAddonPackageResult,
    RemoveAddonRequest, RemovedAddonPackageResult, SearchAddonRequest, UpdateAddonRequest,
    UpdatedAddonPackageResult, install_addon_task_with_provider, list_addons, remove_addons_task,
    search_addons_with_provider, update_addons_task_with_provider,
};
use crate::core::app::{AppRuntime, ListAddonsRequest, task_support};
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

    pub fn search(&self, request: SearchAddonRequest) -> AppResult<AddonSearchCatalog> {
        search_addons_with_provider(self.runtime.addon_provider(), request)
    }

    pub fn list(&self, request: ListAddonsRequest) -> AppResult<AddonInventory> {
        list_addons(&request.installation)
    }

    pub fn install(&self, request: InstallAddonRequest) -> AppResult<InstalledAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_task<TCancel, TProgress>(
        &self,
        request: InstallAddonRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<InstalledAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        install_addon_task_with_provider(
            self.runtime.addon_provider(),
            request,
            cancellation,
            progress,
        )
    }

    pub fn install_collecting_progress(
        &self,
        request: InstallAddonRequest,
    ) -> AppResult<TaskRun<InstalledAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_with_callbacks<FCancel, FProgress>(
        &self,
        request: InstallAddonRequest,
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

    pub fn update(&self, request: UpdateAddonRequest) -> AppResult<UpdatedAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_task<TCancel, TProgress>(
        &self,
        request: UpdateAddonRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<UpdatedAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        update_addons_task_with_provider(
            self.runtime.addon_provider(),
            request,
            cancellation,
            progress,
        )
    }

    pub fn update_collecting_progress(
        &self,
        request: UpdateAddonRequest,
    ) -> AppResult<TaskRun<UpdatedAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_with_callbacks<FCancel, FProgress>(
        &self,
        request: UpdateAddonRequest,
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

    pub fn remove(&self, request: RemoveAddonRequest) -> AppResult<RemovedAddonPackageResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }

    pub fn remove_task<TCancel, TProgress>(
        &self,
        request: RemoveAddonRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<RemovedAddonPackageResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        remove_addons_task(request, cancellation, progress)
    }

    pub fn remove_collecting_progress(
        &self,
        request: RemoveAddonRequest,
    ) -> AppResult<TaskRun<RemovedAddonPackageResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.remove_task(request, cancellation, progress)
        })
    }

    pub fn remove_with_callbacks<FCancel, FProgress>(
        &self,
        request: RemoveAddonRequest,
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
            .install(InstallAddonRequest {
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
            .install_collecting_progress(InstallAddonRequest {
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
            .install(InstallAddonRequest {
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
            inventory.tracked_packages[0].source,
            AddonSourceRef::HttpArchive {
                url: "https://example.invalid/WeakAuras.zip".to_string(),
            }
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
            .install(InstallAddonRequest {
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
                UpdateAddonRequest {
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
            .install(InstallAddonRequest {
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
