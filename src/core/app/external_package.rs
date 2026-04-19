use crate::core::app::{
    AnalyzeExternalPackageAppRequest, AppRuntime, ApplyExternalPackageAppRequest,
    CancellationToken, CreateExternalPackageBundleAppRequest, ExternalPackageAnalysisResult,
    ExternalPackageApplyPlanResult, ExternalPackageApplyResult, ExternalPackageBundleHandle,
    PlanExternalPackageApplyAppRequest, TaskProgressEvent, TaskProgressSink, TaskRun, task_support,
};
use crate::core::bundle::{
    analyze_external_package_task, apply_external_package_task, create_external_package_bundle,
    plan_external_package_apply_task,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub struct ExternalPackageService {
    runtime: AppRuntime,
}

impl ExternalPackageService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn analyze(
        &self,
        request: AnalyzeExternalPackageAppRequest,
    ) -> AppResult<ExternalPackageAnalysisResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.analyze_task(request, cancellation, progress)
        })
    }

    pub fn analyze_task<TCancel, TProgress>(
        &self,
        request: AnalyzeExternalPackageAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageAnalysisResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let analysis = analyze_external_package_task(request.into(), cancellation, progress)?;
        Ok(ExternalPackageAnalysisResult::from_domain(analysis))
    }

    pub fn analyze_collecting_progress(
        &self,
        request: AnalyzeExternalPackageAppRequest,
    ) -> AppResult<TaskRun<ExternalPackageAnalysisResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.analyze_task(request, cancellation, progress)
        })
    }

    pub fn analyze_with_callbacks<FCancel, FProgress>(
        &self,
        request: AnalyzeExternalPackageAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageAnalysisResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.analyze_task(request, cancellation, progress)
        })
    }

    pub fn create_bundle(
        &self,
        request: CreateExternalPackageBundleAppRequest,
    ) -> AppResult<ExternalPackageBundleHandle> {
        let bundle = create_external_package_bundle(request.into_domain_request(&self.runtime))?;
        Ok(ExternalPackageBundleHandle::from_domain(bundle))
    }

    pub fn plan_apply(
        &self,
        request: PlanExternalPackageApplyAppRequest,
    ) -> AppResult<ExternalPackageApplyPlanResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.plan_apply_task(request, cancellation, progress)
        })
    }

    pub fn plan_apply_task<TCancel, TProgress>(
        &self,
        request: PlanExternalPackageApplyAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageApplyPlanResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let plan = plan_external_package_apply_task(
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(ExternalPackageApplyPlanResult::from_domain_plan(
            plan,
            self.runtime.helper_strategy(),
        ))
    }

    pub fn plan_apply_collecting_progress(
        &self,
        request: PlanExternalPackageApplyAppRequest,
    ) -> AppResult<TaskRun<ExternalPackageApplyPlanResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.plan_apply_task(request, cancellation, progress)
        })
    }

    pub fn plan_apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: PlanExternalPackageApplyAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageApplyPlanResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.plan_apply_task(request, cancellation, progress)
        })
    }

    pub fn apply(
        &self,
        request: ApplyExternalPackageAppRequest,
    ) -> AppResult<ExternalPackageApplyResult> {
        task_support::run_direct_task(|cancellation, progress| {
            self.apply_task(request, cancellation, progress)
        })
    }

    pub fn apply_task<TCancel, TProgress>(
        &self,
        request: ApplyExternalPackageAppRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageApplyResult>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        let applied = apply_external_package_task(
            request.into_domain_request(&self.runtime),
            cancellation,
            progress,
        )?;
        Ok(ExternalPackageApplyResult::from_domain(applied))
    }

    pub fn apply_collecting_progress(
        &self,
        request: ApplyExternalPackageAppRequest,
    ) -> AppResult<TaskRun<ExternalPackageApplyResult>> {
        task_support::run_collecting_task(|cancellation, progress| {
            self.apply_task(request, cancellation, progress)
        })
    }

    pub fn apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: ApplyExternalPackageAppRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageApplyResult>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        task_support::run_callback_task(is_cancelled, on_progress, |cancellation, progress| {
            self.apply_task(request, cancellation, progress)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::tempdir;

    use super::*;
    use crate::core::app::{
        AppRuntime, BundleApplyDefaultsValue, BundleApplyMappingsValue, HelperStrategyValue,
        HostPlatformValue, ResolvedInstallationValue, ResourceApplyPolicyValue, WowFlavorValue,
    };
    use crate::core::install::{HostPlatform, WowFlavor};
    use crate::core::task::{NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

    #[test]
    fn external_package_service_analyzes_minimal_source_package() {
        let temp = tempdir().expect("temp dir");
        let package_root = create_minimal_external_package_source(temp.path());

        let service = ExternalPackageService::new();
        let analysis = service
            .analyze(AnalyzeExternalPackageAppRequest {
                source_path: package_root,
            })
            .expect("analyze package");

        assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
        assert_eq!(analysis.summary.warning_count, 0);
    }

    #[test]
    fn external_package_service_analyze_collecting_progress_returns_events() {
        let temp = tempdir().expect("temp dir");
        let package_root = create_minimal_external_package_source(temp.path());

        let service = ExternalPackageService::new();
        let run = service
            .analyze_collecting_progress(AnalyzeExternalPackageAppRequest {
                source_path: package_root,
            })
            .expect("analyze with collected progress");

        assert_eq!(run.result.resources.addons, vec!["WeakAuras".to_string()]);
        assert_eq!(
            run.progress
                .iter()
                .map(|event| (event.task, event.phase))
                .collect::<Vec<_>>(),
            vec![
                (TaskKind::ExternalPackageAnalyze, TaskPhase::Preparing),
                (TaskKind::ExternalPackageAnalyze, TaskPhase::Planning),
                (TaskKind::ExternalPackageAnalyze, TaskPhase::Completed),
            ]
        );
    }

    #[test]
    fn external_package_service_analyze_with_callbacks_uses_plain_closures() {
        let temp = tempdir().expect("temp dir");
        let package_root = create_minimal_external_package_source(temp.path());

        let service = ExternalPackageService::new();
        let seen = RefCell::new(Vec::new());
        let cancellation_checks = Cell::new(0usize);
        let analysis = service
            .analyze_with_callbacks(
                AnalyzeExternalPackageAppRequest {
                    source_path: package_root,
                },
                || {
                    let next = cancellation_checks.get() + 1;
                    cancellation_checks.set(next);
                    false
                },
                |event| seen.borrow_mut().push(event),
            )
            .expect("analyze with callbacks");

        assert_eq!(analysis.summary.warning_count, 0);
        assert_eq!(seen.borrow().len(), 3);
        assert!(cancellation_checks.get() >= 2);
    }

    #[test]
    fn external_package_service_apply_task_uses_external_package_task_kind() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let package_root = create_minimal_external_package_source(source.path());
        let target_installation = create_empty_installation(target.path());

        let service = ExternalPackageService::new();
        let cancellation = NeverCancel;
        let mut progress = VecTaskProgressSink::default();
        let result = service
            .apply_task(
                ApplyExternalPackageAppRequest {
                    external_package: CreateExternalPackageBundleAppRequest {
                        source_path: package_root,
                        source_flavor: WowFlavorValue::Retail,
                        source_platform: Some(HostPlatformValue::Windows),
                        supported_targets: vec![WowFlavorValue::Retail],
                        output_path: None,
                        package_id: None,
                        package_name: None,
                        created_by: None,
                        description: None,
                        apply_defaults: None,
                    },
                    installation: target_installation.clone(),
                    dry_run: true,
                    backup_output_path: None,
                    apply_mappings: BundleApplyMappingsValue::default(),
                },
                &cancellation,
                &mut progress,
            )
            .expect("apply task");

        assert!(result.dry_run);
        assert_eq!(
            progress
                .events()
                .iter()
                .map(|event| (event.task, event.phase))
                .collect::<Vec<_>>(),
            vec![
                (TaskKind::ExternalPackageApply, TaskPhase::Preparing),
                (TaskKind::ExternalPackageApply, TaskPhase::Planning),
                (TaskKind::ExternalPackageApply, TaskPhase::Completed),
            ]
        );
    }

    #[test]
    fn external_package_service_plan_apply_reports_runtime_helper_strategy() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let package_root = create_minimal_external_package_source(source.path());
        let target_installation = create_empty_installation(target.path());

        let plan = ExternalPackageService::new()
            .plan_apply(PlanExternalPackageApplyAppRequest {
                external_package: CreateExternalPackageBundleAppRequest {
                    source_path: package_root,
                    source_flavor: WowFlavorValue::Retail,
                    source_platform: Some(HostPlatformValue::Windows),
                    supported_targets: vec![WowFlavorValue::Retail],
                    output_path: None,
                    package_id: None,
                    package_name: None,
                    created_by: None,
                    description: None,
                    apply_defaults: None,
                },
                installation: target_installation,
                apply_mappings: BundleApplyMappingsValue::default(),
            })
            .expect("plan apply");

        assert_eq!(plan.helper_strategy, HelperStrategyValue::NativeRust);
    }

    #[test]
    fn external_package_service_apply_collecting_progress_returns_external_task_events() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let package_root = create_minimal_external_package_source(source.path());
        let target_installation = create_empty_installation(target.path());

        let service = ExternalPackageService::new();
        let run = service
            .apply_collecting_progress(ApplyExternalPackageAppRequest {
                external_package: CreateExternalPackageBundleAppRequest {
                    source_path: package_root,
                    source_flavor: WowFlavorValue::Retail,
                    source_platform: Some(HostPlatformValue::Windows),
                    supported_targets: vec![WowFlavorValue::Retail],
                    output_path: None,
                    package_id: None,
                    package_name: None,
                    created_by: None,
                    description: None,
                    apply_defaults: None,
                },
                installation: target_installation,
                dry_run: true,
                backup_output_path: None,
                apply_mappings: BundleApplyMappingsValue::default(),
            })
            .expect("apply with collected progress");

        assert!(run.result.dry_run);
        assert_eq!(
            run.progress
                .iter()
                .map(|event| event.task)
                .collect::<Vec<_>>(),
            vec![
                TaskKind::ExternalPackageApply,
                TaskKind::ExternalPackageApply,
                TaskKind::ExternalPackageApply,
            ]
        );
    }

    #[test]
    fn external_package_service_create_bundle_uses_runtime_platform_and_output_dir() {
        let source = tempdir().expect("source temp dir");
        let output = tempdir().expect("output temp dir");
        let package_root = create_minimal_external_package_source(source.path());

        let service = ExternalPackageService::with_runtime(
            AppRuntime::new()
                .with_host_platform(HostPlatformValue::MacOs)
                .with_default_bundle_output_dir(Some(output.path().to_path_buf())),
        );
        let prepared = service
            .create_bundle(CreateExternalPackageBundleAppRequest {
                source_path: package_root,
                source_flavor: WowFlavorValue::Retail,
                source_platform: None,
                supported_targets: vec![WowFlavorValue::Retail],
                output_path: None,
                package_id: None,
                package_name: None,
                created_by: None,
                description: None,
                apply_defaults: Some(BundleApplyDefaultsValue {
                    create_backup: false,
                    addons: ResourceApplyPolicyValue::Mirror,
                    wtf_common: ResourceApplyPolicyValue::Share,
                    wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
                    fonts: ResourceApplyPolicyValue::Preserve,
                    interface_assets: ResourceApplyPolicyValue::Mirror,
                }),
            })
            .expect("create bundle with runtime defaults");

        assert_eq!(
            prepared.manifest().source.platform,
            Some(HostPlatformValue::MacOs)
        );
        assert!(!prepared.manifest().apply.create_backup);
        assert_eq!(prepared.bundle().archive_path.parent(), Some(output.path()));
        assert!(prepared.archive_path().is_file());
    }

    #[test]
    fn external_package_service_create_bundle_keeps_temporary_bundle_alive_while_handle_exists() {
        let source = tempdir().expect("source temp dir");
        let package_root = create_minimal_external_package_source(source.path());

        let service = ExternalPackageService::new();
        let prepared = service
            .create_bundle(CreateExternalPackageBundleAppRequest {
                source_path: package_root,
                source_flavor: WowFlavorValue::Retail,
                source_platform: None,
                supported_targets: vec![WowFlavorValue::Retail],
                output_path: None,
                package_id: None,
                package_name: None,
                created_by: None,
                description: None,
                apply_defaults: None,
            })
            .expect("create temporary bundle");

        let archive_path = prepared.archive_path().to_path_buf();
        assert!(archive_path.is_file());

        drop(prepared);

        assert!(!archive_path.exists());
    }

    fn create_minimal_external_package_source(root: &Path) -> PathBuf {
        let package_root = root.join("AuthorPack");
        let addon_root = package_root.join("WeakAuras");
        fs::create_dir_all(&addon_root).expect("addon dir");
        fs::write(
            addon_root.join("WeakAuras.toc"),
            "## Interface: 110000\n## Title: WeakAuras\n",
        )
        .expect("toc");
        fs::write(addon_root.join("WeakAuras.lua"), "WeakAurasSaved = {}").expect("lua");
        package_root
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

        crate::core::install::DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root,
            flavor_root,
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        }
        .into()
    }
}
