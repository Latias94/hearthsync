use crate::core::bundle::{
    AnalyzeExternalPackageRequest, AppliedExternalPackage, ApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest, ExternalPackageAnalysis, ExternalPackageApplyPlan,
    PlanExternalPackageApplyRequest, PreparedExternalPackageBundle, analyze_external_package,
    analyze_external_package_task, apply_external_package, apply_external_package_task,
    create_external_package_bundle, plan_external_package_apply, plan_external_package_apply_task,
};
use crate::core::error::AppResult;
use crate::core::task::{
    CancellationToken, TaskProgressEvent, TaskProgressSink, TaskRun, run_task_with_callbacks,
    run_task_with_collected_progress,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct ExternalPackageService;

impl ExternalPackageService {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(
        &self,
        request: AnalyzeExternalPackageRequest,
    ) -> AppResult<ExternalPackageAnalysis> {
        analyze_external_package(request)
    }

    pub fn analyze_task<TCancel, TProgress>(
        &self,
        request: AnalyzeExternalPackageRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageAnalysis>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        analyze_external_package_task(request, cancellation, progress)
    }

    pub fn analyze_collecting_progress(
        &self,
        request: AnalyzeExternalPackageRequest,
    ) -> AppResult<TaskRun<ExternalPackageAnalysis>> {
        run_task_with_collected_progress(|cancellation, progress| {
            self.analyze_task(request, cancellation, progress)
        })
    }

    pub fn analyze_with_callbacks<FCancel, FProgress>(
        &self,
        request: AnalyzeExternalPackageRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageAnalysis>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        run_task_with_callbacks(is_cancelled, on_progress, |cancellation, progress| {
            self.analyze_task(request, cancellation, progress)
        })
    }

    pub fn create_bundle(
        &self,
        request: CreateExternalPackageBundleRequest,
    ) -> AppResult<PreparedExternalPackageBundle> {
        create_external_package_bundle(request)
    }

    pub fn plan_apply(
        &self,
        request: PlanExternalPackageApplyRequest,
    ) -> AppResult<ExternalPackageApplyPlan> {
        plan_external_package_apply(request)
    }

    pub fn plan_apply_task<TCancel, TProgress>(
        &self,
        request: PlanExternalPackageApplyRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<ExternalPackageApplyPlan>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        plan_external_package_apply_task(request, cancellation, progress)
    }

    pub fn plan_apply_collecting_progress(
        &self,
        request: PlanExternalPackageApplyRequest,
    ) -> AppResult<TaskRun<ExternalPackageApplyPlan>> {
        run_task_with_collected_progress(|cancellation, progress| {
            self.plan_apply_task(request, cancellation, progress)
        })
    }

    pub fn plan_apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: PlanExternalPackageApplyRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<ExternalPackageApplyPlan>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        run_task_with_callbacks(is_cancelled, on_progress, |cancellation, progress| {
            self.plan_apply_task(request, cancellation, progress)
        })
    }

    pub fn apply(&self, request: ApplyExternalPackageRequest) -> AppResult<AppliedExternalPackage> {
        apply_external_package(request)
    }

    pub fn apply_task<TCancel, TProgress>(
        &self,
        request: ApplyExternalPackageRequest,
        cancellation: &TCancel,
        progress: &mut TProgress,
    ) -> AppResult<AppliedExternalPackage>
    where
        TCancel: CancellationToken,
        TProgress: TaskProgressSink,
    {
        apply_external_package_task(request, cancellation, progress)
    }

    pub fn apply_collecting_progress(
        &self,
        request: ApplyExternalPackageRequest,
    ) -> AppResult<TaskRun<AppliedExternalPackage>> {
        run_task_with_collected_progress(|cancellation, progress| {
            self.apply_task(request, cancellation, progress)
        })
    }

    pub fn apply_with_callbacks<FCancel, FProgress>(
        &self,
        request: ApplyExternalPackageRequest,
        is_cancelled: FCancel,
        on_progress: FProgress,
    ) -> AppResult<AppliedExternalPackage>
    where
        FCancel: Fn() -> bool,
        FProgress: FnMut(TaskProgressEvent),
    {
        run_task_with_callbacks(is_cancelled, on_progress, |cancellation, progress| {
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
    use crate::core::bundle::BundleApplyMappings;
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
    use crate::core::task::{NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

    #[test]
    fn external_package_service_analyzes_minimal_source_package() {
        let temp = tempdir().expect("temp dir");
        let package_root = create_minimal_external_package_source(temp.path());

        let service = ExternalPackageService::new();
        let analysis = service
            .analyze(AnalyzeExternalPackageRequest {
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
            .analyze_collecting_progress(AnalyzeExternalPackageRequest {
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
                AnalyzeExternalPackageRequest {
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
                ApplyExternalPackageRequest {
                    external_package: CreateExternalPackageBundleRequest {
                        source_path: package_root,
                        source_flavor: WowFlavor::Retail,
                        source_platform: Some(HostPlatform::Windows),
                        supported_targets: vec![WowFlavor::Retail],
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
                    apply_mappings: BundleApplyMappings::default(),
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
    fn external_package_service_apply_collecting_progress_returns_external_task_events() {
        let source = tempdir().expect("source temp dir");
        let target = tempdir().expect("target temp dir");
        let package_root = create_minimal_external_package_source(source.path());
        let target_installation = create_empty_installation(target.path());

        let service = ExternalPackageService::new();
        let run = service
            .apply_collecting_progress(ApplyExternalPackageRequest {
                external_package: CreateExternalPackageBundleRequest {
                    source_path: package_root,
                    source_flavor: WowFlavor::Retail,
                    source_platform: Some(HostPlatform::Windows),
                    supported_targets: vec![WowFlavor::Retail],
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
                apply_mappings: BundleApplyMappings::default(),
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
}
