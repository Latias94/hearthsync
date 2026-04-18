use crate::core::app::{AppRuntime, task_support};
use crate::core::app::{InspectBundleRequest, PlanBundleAddonLockRequest, PlanBundleApplyRequest};
use crate::core::bundle::{
    BundleAddonLockApply, BundleAddonLockApplyRequest, BundleAddonLockPlan, BundleApplyPlan,
    BundleInspection, CreatedBundle, PackBundleRequest, UnpackBundleRequest, UnpackedBundle,
    apply_bundle_addon_lock, inspect_bundle, pack_bundle, plan_bundle_addon_lock,
    plan_bundle_apply, unpack_bundle_task,
};
use crate::core::error::AppResult;
use crate::core::task::{CancellationToken, TaskProgressEvent, TaskProgressSink, TaskRun};

#[derive(Debug, Clone, Default)]
pub struct BundleService {
    runtime: AppRuntime,
}

impl BundleService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn inspect(&self, request: InspectBundleRequest) -> AppResult<BundleInspection> {
        inspect_bundle(&request.bundle_path)
    }

    pub fn pack(&self, request: PackBundleRequest) -> AppResult<CreatedBundle> {
        pack_bundle(self.normalize_pack_request(request))
    }

    pub fn plan_apply(&self, request: PlanBundleApplyRequest) -> AppResult<BundleApplyPlan> {
        plan_bundle_apply(
            &request.bundle_path,
            &request.installation,
            &request.apply_mappings,
        )
    }

    pub fn apply(&self, request: UnpackBundleRequest) -> AppResult<UnpackedBundle> {
        task_support::run_direct_task(|cancellation, progress| {
            self.apply_task(request, cancellation, progress)
        })
    }

    pub fn plan_addon_lock(
        &self,
        request: PlanBundleAddonLockRequest,
    ) -> AppResult<BundleAddonLockPlan> {
        plan_bundle_addon_lock(&request.bundle_path, &request.installation)
    }

    pub fn apply_addon_lock(
        &self,
        request: BundleAddonLockApplyRequest,
    ) -> AppResult<BundleAddonLockApply> {
        apply_bundle_addon_lock(self.normalize_addon_lock_request(request))
    }

    pub fn apply_task<TCancel, TProgress>(
        &self,
        request: UnpackBundleRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<UnpackedBundle>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        unpack_bundle_task(
            self.normalize_unpack_request(request),
            cancellation,
            progress,
        )
    }

    pub fn apply_collecting_progress(
        &self,
        request: UnpackBundleRequest,
    ) -> AppResult<TaskRun<UnpackedBundle>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.apply_task(request, cancellation, progress)
        })
    }

    pub fn apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: UnpackBundleRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<UnpackedBundle>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.apply_task(request, cancellation, progress)
        })
    }

    fn normalize_pack_request(&self, mut request: PackBundleRequest) -> PackBundleRequest {
        request.output_path = self.runtime.bundle_output_or_default(request.output_path);
        request
    }

    fn normalize_unpack_request(&self, mut request: UnpackBundleRequest) -> UnpackBundleRequest {
        request.backup_output_path = self
            .runtime
            .backup_output_or_default(request.backup_output_path);
        request
    }

    fn normalize_addon_lock_request(
        &self,
        mut request: BundleAddonLockApplyRequest,
    ) -> BundleAddonLockApplyRequest {
        request.backup_output_path = self
            .runtime
            .backup_output_or_default(request.backup_output_path);
        request
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::core::app::AppRuntime;
    use crate::core::bundle::BundleApplyMappings;
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
    use crate::core::manifest::{
        ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, MappingRules,
        PackageMetadata, ResourceApplyPolicy, SourceInstallation,
    };
    use crate::core::task::{TaskKind, TaskPhase};

    #[test]
    fn bundle_service_plan_apply_reads_bundle_plan() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let source_installation = create_bundle_fixture_installation(source.path(), true);
        let target_installation = create_bundle_fixture_installation(target.path(), false);
        let bundle_path = source.path().join("fixture.bundle.zip");

        let service = BundleService::new();
        service
            .pack(PackBundleRequest {
                installation: source_installation,
                manifest: sample_bundle_manifest(),
                output_path: Some(bundle_path.clone()),
                manifest_base_dir: None,
            })
            .expect("pack bundle");

        let plan = service
            .plan_apply(PlanBundleApplyRequest {
                bundle_path: bundle_path.clone(),
                installation: target_installation,
                apply_mappings: BundleApplyMappings::default(),
            })
            .expect("plan bundle apply");

        assert_eq!(plan.bundle_path, bundle_path);
        assert!(
            plan.operations
                .iter()
                .any(|item| item.group == crate::core::bundle::ApplyGroup::Addons)
        );
    }

    #[test]
    fn bundle_service_apply_collecting_progress_returns_bundle_task_events() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let source_installation = create_bundle_fixture_installation(source.path(), true);
        let target_installation = create_bundle_fixture_installation(target.path(), false);
        let bundle_path = source.path().join("fixture.bundle.zip");

        let service = BundleService::new();
        service
            .pack(PackBundleRequest {
                installation: source_installation,
                manifest: sample_bundle_manifest(),
                output_path: Some(bundle_path.clone()),
                manifest_base_dir: None,
            })
            .expect("pack bundle");

        let run = service
            .apply_collecting_progress(UnpackBundleRequest {
                bundle_path,
                installation: target_installation,
                dry_run: true,
                backup_output_path: None,
                apply_mappings: BundleApplyMappings::default(),
            })
            .expect("apply bundle with progress");

        assert!(run.result.dry_run);
        assert_eq!(
            run.progress
                .iter()
                .map(|event| (event.task, event.phase))
                .collect::<Vec<_>>(),
            vec![
                (TaskKind::BundleApply, TaskPhase::Preparing),
                (TaskKind::BundleApply, TaskPhase::Planning),
                (TaskKind::BundleApply, TaskPhase::Completed),
            ]
        );
    }

    #[test]
    fn bundle_service_pack_uses_runtime_default_output_dir() {
        let source = tempdir().expect("source temp dir");
        let output = tempdir().expect("output temp dir");
        let source_installation = create_bundle_fixture_installation(source.path(), true);

        let service = BundleService::with_runtime(
            AppRuntime::new().with_default_bundle_output_dir(Some(output.path().to_path_buf())),
        );
        let created = service
            .pack(PackBundleRequest {
                installation: source_installation,
                manifest: sample_bundle_manifest(),
                output_path: None,
                manifest_base_dir: None,
            })
            .expect("pack bundle with runtime output dir");

        assert_eq!(created.archive_path.parent(), Some(output.path()));
        assert!(created.archive_path.is_file());
    }

    #[test]
    fn bundle_service_apply_uses_runtime_default_backup_dir() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let backup = tempdir().expect("backup temp dir");
        let source_installation = create_bundle_fixture_installation(source.path(), true);
        let target_installation = create_bundle_fixture_installation(target.path(), false);
        let bundle_path = source.path().join("fixture.bundle.zip");

        let service = BundleService::with_runtime(
            AppRuntime::new().with_default_backup_dir(Some(backup.path().to_path_buf())),
        );
        service
            .pack(PackBundleRequest {
                installation: source_installation,
                manifest: sample_bundle_manifest(),
                output_path: Some(bundle_path.clone()),
                manifest_base_dir: None,
            })
            .expect("pack bundle");

        let applied = service
            .apply(UnpackBundleRequest {
                bundle_path,
                installation: target_installation,
                dry_run: false,
                backup_output_path: None,
                apply_mappings: BundleApplyMappings::default(),
            })
            .expect("apply bundle with runtime backup dir");

        assert_eq!(
            applied.backup_path.as_deref().and_then(Path::parent),
            Some(backup.path())
        );
    }

    fn create_bundle_fixture_installation(
        root: &Path,
        with_content: bool,
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

        if with_content {
            fs::create_dir_all(addon_dir.join("WeakAuras")).expect("weak auras dir");
            fs::write(
                addon_dir.join("WeakAuras").join("WeakAuras.toc"),
                "## Interface: 110000\n",
            )
            .expect("toc");
            fs::write(
                addon_dir.join("WeakAuras").join("WeakAuras.lua"),
                "WeakAurasSaved = {}",
            )
            .expect("lua");
        }

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

    fn sample_bundle_manifest() -> BundleManifest {
        BundleManifest {
            schema_version: 1,
            package: PackageMetadata {
                id: "test-ui".to_string(),
                name: "Test UI".to_string(),
                created_by: "test".to_string(),
                description: None,
            },
            source: SourceInstallation {
                flavor: WowFlavor::Retail,
                platform: None,
                exported_at: None,
                supported_targets: vec![WowFlavor::Retail],
            },
            resources: BundleResources {
                addons: vec!["WeakAuras".to_string()],
                wtf_common: false,
                wtf_characters: Vec::new(),
                fonts: false,
                interface_assets: Vec::new(),
                addon_lock: false,
                addon_indexes: Vec::new(),
            },
            mapping: MappingRules {
                character_mode: CharacterMappingMode::KeepOriginal,
                rewrite_profile_keys: false,
                rewrite_identity_strings: false,
                allow_cross_platform: true,
            },
            apply: ApplyDefaults {
                create_backup: true,
                addons: ResourceApplyPolicy::Merge,
                wtf_common: ResourceApplyPolicy::Merge,
                wtf_characters: ResourceApplyPolicy::Merge,
                fonts: ResourceApplyPolicy::Merge,
                interface_assets: ResourceApplyPolicy::Merge,
            },
        }
    }
}
