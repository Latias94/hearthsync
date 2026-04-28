use std::cell::{Cell, RefCell};
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::core::app::{
    AppRuntime, ApplyConfigAppRequest, BundleApplyMappingsValue, ConfigPackageAppRequest,
    ConfigService, ExternalPackageService, HostPlatformValue, InspectConfigAppRequest,
    ResolvedInstallationValue, WowFlavorValue,
};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::{TaskKind, TaskPhase};

#[test]
fn config_service_inspect_collecting_progress_returns_config_task_events() {
    let temp = tempdir().expect("temp dir");
    let package_root = create_minimal_config_source(temp.path());

    let service = ConfigService::default();
    let run = service
        .inspect_collecting_progress(InspectConfigAppRequest {
            source_path: package_root,
        })
        .expect("inspect with collected progress");

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
fn config_service_inspects_relative_source_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let package_root = create_minimal_config_source(temp.path());

    let service = ConfigService::with_external_packages(ExternalPackageService::with_runtime(
        AppRuntime::new().with_relative_path_base(Some(temp.path().to_path_buf())),
    ));
    let result = service
        .inspect(InspectConfigAppRequest {
            source_path: PathBuf::from("AuthorPack"),
        })
        .expect("inspect relative config source");

    assert_eq!(result.source_path, package_root);
    assert_eq!(result.resources.addons, vec!["WeakAuras".to_string()]);
}

#[test]
fn config_service_apply_with_callbacks_uses_config_facade_requests() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_root = create_minimal_config_source(source.path());
    let installation = create_empty_installation(target.path());

    let service = ConfigService::default();
    let seen = RefCell::new(Vec::new());
    let cancellation_checks = Cell::new(0usize);
    let result = service
        .apply_with_callbacks(
            ApplyConfigAppRequest {
                config_package: sample_config_package(package_root),
                installation,
                dry_run: true,
                backup_output_path: None,
                apply_mappings: BundleApplyMappingsValue::default(),
            },
            || {
                let next = cancellation_checks.get() + 1;
                cancellation_checks.set(next);
                false
            },
            |event| seen.borrow_mut().push(event),
        )
        .expect("apply with callbacks");

    assert!(result.dry_run);
    assert_eq!(seen.borrow().len(), 3);
    assert!(cancellation_checks.get() >= 2);
}

fn sample_config_package(source_path: PathBuf) -> ConfigPackageAppRequest {
    ConfigPackageAppRequest {
        source_path,
        source_flavor: WowFlavorValue::Retail,
        source_platform: Some(HostPlatformValue::Windows),
        supported_targets: vec![WowFlavorValue::Retail],
        output_path: None,
        package_id: None,
        package_name: None,
        created_by: None,
        description: None,
        apply_defaults: None,
    }
}

fn create_minimal_config_source(root: &Path) -> PathBuf {
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
