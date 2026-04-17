use std::path::Path;

use crate::core::addon::index::{
    AddonIndexInspection, AddonIndexInstallRequest, AddonIndexInstallResult,
    AddonIndexUpdateRequest, AddonIndexUpdateResult, inspect_addon_index, install_addon_from_index,
    install_addon_from_index_task, update_addons_from_index, update_addons_from_index_task,
};
use crate::core::error::AppResult;
use crate::core::task::{
    CancellationToken, TaskProgressEvent, TaskProgressSink, TaskRun, run_task_with_callbacks,
    run_task_with_collected_progress,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct AddonIndexService;

impl AddonIndexService {
    pub fn new() -> Self {
        Self
    }

    pub fn inspect(&self, index_path: &Path) -> AppResult<AddonIndexInspection> {
        inspect_addon_index(index_path)
    }

    pub fn install(&self, request: AddonIndexInstallRequest) -> AppResult<AddonIndexInstallResult> {
        install_addon_from_index(request)
    }

    pub fn install_task<TCancel, TProgress>(
        &self,
        request: AddonIndexInstallRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonIndexInstallResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        install_addon_from_index_task(request, cancellation, progress)
    }

    pub fn install_collecting_progress(
        &self,
        request: AddonIndexInstallRequest,
    ) -> AppResult<TaskRun<AddonIndexInstallResult>> {
        run_task_with_collected_progress(|cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn install_with_callbacks<FCancel, FProgress>(
        &self,
        request: AddonIndexInstallRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexInstallResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        run_task_with_callbacks(is_cancelled, on_progress, |cancellation, progress| {
            self.install_task(request, cancellation, progress)
        })
    }

    pub fn update(&self, request: AddonIndexUpdateRequest) -> AppResult<AddonIndexUpdateResult> {
        update_addons_from_index(request)
    }

    pub fn update_task<TCancel, TProgress>(
        &self,
        request: AddonIndexUpdateRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AddonIndexUpdateResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        update_addons_from_index_task(request, cancellation, progress)
    }

    pub fn update_collecting_progress(
        &self,
        request: AddonIndexUpdateRequest,
    ) -> AppResult<TaskRun<AddonIndexUpdateResult>> {
        run_task_with_collected_progress(|cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }

    pub fn update_with_callbacks<FCancel, FProgress>(
        &self,
        request: AddonIndexUpdateRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AddonIndexUpdateResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        run_task_with_callbacks(is_cancelled, on_progress, |cancellation, progress| {
            self.update_task(request, cancellation, progress)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;
    use crate::core::addon::InstallAddonRequest;
    use crate::core::addon::install_addon;
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
    use crate::core::task::{TaskKind, TaskPhase};

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
        let inspection = service.inspect(&index_path).expect("inspect addon index");

        assert_eq!(inspection.package_count, 1);
        assert_eq!(inspection.index.name, "Fixture Index");
        assert_eq!(inspection.index.packages[0].id, "weakauras");
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
            .install_collecting_progress(AddonIndexInstallRequest {
                installation,
                index_path,
                name: "weakauras".to_string(),
                dry_run: false,
                backup_output_path: Some(temp.path().join("backups")),
                replace_existing: false,
            })
            .expect("install from index with collected progress");

        assert_eq!(run.result.package.id, "weakauras");
        assert_eq!(
            run.progress
                .iter()
                .map(|event| (event.task, event.phase))
                .collect::<Vec<_>>(),
            vec![
                (TaskKind::AddonIndexInstall, TaskPhase::Preparing),
                (TaskKind::AddonIndexInstall, TaskPhase::BackingUp),
                (TaskKind::AddonIndexInstall, TaskPhase::Executing),
                (TaskKind::AddonIndexInstall, TaskPhase::Completed),
            ]
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
            installation: installation.clone(),
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
            AddonIndexUpdateRequest {
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
        assert_eq!(seen.borrow().len(), 4);
        assert!(cancellation_checks.get() >= 3);
    }

    fn service_update_with_callbacks(
        service: &AddonIndexService,
        request: AddonIndexUpdateRequest,
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
}
