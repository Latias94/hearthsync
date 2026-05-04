use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::core::app::{
    AnalyzeExternalPackageAppRequest, AppRuntime, ApplyExternalPackageAppRequest,
    BundleApplyDefaultsValue, BundleApplyMappingsValue, CreateExternalPackageBundleAppRequest,
    ExternalPackageLayoutValue, ExternalPackageService, HelperStrategyValue, HostPlatformValue,
    PlanExternalPackageApplyAppRequest, ResolvedInstallationValue, ResourceApplyPolicyValue,
    WowFlavorValue,
};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{NeverCancel, TaskKind, TaskPhase, TaskProgressCode, VecTaskProgressSink};

#[test]
fn external_package_service_analyzes_minimal_source_package() {
    let temp = tempdir().expect("temp dir");
    let package_root = create_minimal_external_package_source(temp.path());

    let service = ExternalPackageService::new();
    let analysis = service
        .analyze_collecting_progress(AnalyzeExternalPackageAppRequest {
            source_path: package_root,
            layout: ExternalPackageLayoutValue::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
        })
        .expect("analyze package")
        .result;

    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert_eq!(analysis.summary.warning_count, 0);
}

#[test]
fn external_package_service_analyzes_relative_source_against_runtime_base() {
    let source = tempdir().expect("source temp dir");
    let package_root = create_minimal_external_package_source(source.path());

    let service = ExternalPackageService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(source.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    let analysis = service
        .analyze_collecting_progress(AnalyzeExternalPackageAppRequest {
            source_path: PathBuf::from("AuthorPack"),
            layout: ExternalPackageLayoutValue::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
        })
        .expect("analyze relative package source")
        .result;

    assert_eq!(analysis.source_path, package_root);
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
}

#[test]
fn external_package_service_rejects_relative_source_without_runtime_base() {
    let error = ExternalPackageService::new()
        .analyze_collecting_progress(AnalyzeExternalPackageAppRequest {
            source_path: PathBuf::from("AuthorPack"),
            layout: ExternalPackageLayoutValue::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
        })
        .expect_err("relative package source without base should fail");

    assert!(error.to_string().contains("relative path base"));
}

#[test]
fn external_package_service_analyze_collecting_progress_returns_events() {
    let temp = tempdir().expect("temp dir");
    let package_root = create_minimal_external_package_source(temp.path());

    let service = ExternalPackageService::new();
    let run = service
        .analyze_collecting_progress(AnalyzeExternalPackageAppRequest {
            source_path: package_root,
            layout: ExternalPackageLayoutValue::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
        })
        .expect("analyze with collected progress");

    assert_eq!(run.result.resources.addons, vec!["WeakAuras".to_string()]);
    assert!(run.task_id.starts_with("task-"));
    assert!(
        run.progress
            .iter()
            .all(|event| event.task_id.as_deref() == Some(run.task_id.as_str()))
    );
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
    assert_eq!(
        run.progress
            .iter()
            .map(|event| event.code)
            .collect::<Vec<_>>(),
        vec![
            Some(TaskProgressCode::Preparing),
            Some(TaskProgressCode::Planning),
            Some(TaskProgressCode::Completed),
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
                layout: ExternalPackageLayoutValue::Auto,
                source_account: None,
                source_server: None,
                source_character: None,
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
                    layout: ExternalPackageLayoutValue::Auto,
                    source_account: None,
                    source_server: None,
                    source_character: None,
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
        .plan_apply_collecting_progress(PlanExternalPackageApplyAppRequest {
            external_package: CreateExternalPackageBundleAppRequest {
                source_path: package_root,
                layout: ExternalPackageLayoutValue::Auto,
                source_account: None,
                source_server: None,
                source_character: None,
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
        .expect("plan apply")
        .result;

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
                layout: ExternalPackageLayoutValue::Auto,
                source_account: None,
                source_server: None,
                source_character: None,
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
        AppRuntime::builder()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_default_bundle_output_dir(Some(output.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    let prepared = service
        .create_bundle(CreateExternalPackageBundleAppRequest {
            source_path: package_root,
            layout: ExternalPackageLayoutValue::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
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
    let prepared = prepared.as_ref();

    assert_eq!(
        prepared.manifest.source.platform,
        Some(HostPlatformValue::MacOs)
    );
    assert!(!prepared.manifest.apply.create_backup);
    assert_eq!(prepared.bundle.archive_path.parent(), Some(output.path()));
    assert!(prepared.bundle.archive_path.is_file());
}

#[test]
fn external_package_service_create_bundle_resolves_relative_runtime_output_dir() {
    let source = tempdir().expect("source temp dir");
    let output_base = tempdir().expect("output base temp dir");
    let package_root = create_minimal_external_package_source(source.path());

    let service = ExternalPackageService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(output_base.path().to_path_buf()))
            .with_default_bundle_output_dir(Some(PathBuf::from("exports")))
            .build()
            .expect("runtime"),
    );
    let prepared = service
        .create_bundle(CreateExternalPackageBundleAppRequest {
            source_path: package_root,
            layout: ExternalPackageLayoutValue::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
            source_flavor: WowFlavorValue::Retail,
            source_platform: Some(HostPlatformValue::Windows),
            supported_targets: vec![WowFlavorValue::Retail],
            output_path: None,
            package_id: None,
            package_name: None,
            created_by: None,
            description: None,
            apply_defaults: None,
        })
        .expect("create bundle with relative runtime output dir");

    let expected_output = output_base.path().join("exports");
    assert_eq!(
        prepared.as_ref().bundle.archive_path.parent(),
        Some(expected_output.as_path())
    );
}

#[test]
fn external_package_service_create_bundle_rejects_relative_runtime_output_without_base() {
    let error = AppRuntime::builder()
        .with_default_bundle_output_dir(Some(PathBuf::from("exports")))
        .build()
        .expect_err("relative runtime output without base should fail");

    assert!(error.to_string().contains("relative path base"));
}

#[test]
fn external_package_service_create_bundle_keeps_temporary_bundle_alive_while_handle_exists() {
    let source = tempdir().expect("source temp dir");
    let package_root = create_minimal_external_package_source(source.path());

    let service = ExternalPackageService::new();
    let prepared = service
        .create_bundle(CreateExternalPackageBundleAppRequest {
            source_path: package_root,
            layout: ExternalPackageLayoutValue::Auto,
            source_account: None,
            source_server: None,
            source_character: None,
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

    let archive_path = prepared.as_ref().bundle.archive_path.clone();
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
