use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::core::addon::AddonStatePaths;
use crate::core::app::{
    AddonService, AppRuntime, ApplyBundleAddonLockAppRequest, ApplyBundleAppRequest,
    BundleApplyDefaultsValue, BundleApplyMappingsValue, BundleManifestValue,
    BundleMappingRulesValue, BundlePackageValue, BundleResourcesValue, BundleService,
    BundleSourceValue, CharacterMappingModeValue, HelperStrategyValue, InspectBundleRequest,
    InstallAddonAppRequest, ListAddonsRequest, PackBundleAppRequest, PlanBundleApplyRequest,
    ResolvedInstallationValue, ResourceApplyPolicyValue, WowFlavorValue,
};
use crate::core::error::AppError;
use crate::core::install::{HostPlatform, WowFlavor};
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
        .pack(PackBundleAppRequest {
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
            apply_mappings: BundleApplyMappingsValue::default(),
        })
        .expect("plan bundle apply");

    assert_eq!(plan.bundle_path, bundle_path);
    assert_eq!(plan.helper_strategy, HelperStrategyValue::NativeRust);
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == crate::core::app::ApplyGroupValue::Addons)
    );
}

#[test]
fn bundle_service_inspects_relative_bundle_against_runtime_base() {
    let source = tempdir().expect("source temp dir");
    let source_installation = create_bundle_fixture_installation(source.path(), true);
    let bundle_path = source.path().join("fixture.bundle.zip");

    BundleService::new()
        .pack(PackBundleAppRequest {
            installation: source_installation,
            manifest: sample_bundle_manifest(),
            output_path: Some(bundle_path.clone()),
            manifest_base_dir: None,
        })
        .expect("pack bundle");

    let service = BundleService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(source.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    let inspection = service
        .inspect(InspectBundleRequest {
            bundle_path: PathBuf::from("fixture.bundle.zip"),
        })
        .expect("inspect relative bundle");

    assert_eq!(inspection.archive_path, bundle_path);
}

#[test]
fn bundle_service_rejects_relative_bundle_without_runtime_base() {
    let error = BundleService::new()
        .inspect(InspectBundleRequest {
            bundle_path: PathBuf::from("fixture.bundle.zip"),
        })
        .expect_err("relative bundle without base should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("relative path base"));
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
        .pack(PackBundleAppRequest {
            installation: source_installation,
            manifest: sample_bundle_manifest(),
            output_path: Some(bundle_path.clone()),
            manifest_base_dir: None,
        })
        .expect("pack bundle");

    let run = service
        .apply_collecting_progress(ApplyBundleAppRequest {
            bundle_path,
            installation: target_installation,
            dry_run: true,
            backup_output_path: None,
            apply_mappings: BundleApplyMappingsValue::default(),
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
        AppRuntime::builder()
            .with_default_bundle_output_dir(Some(output.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    let created = service
        .pack(PackBundleAppRequest {
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
        AppRuntime::builder()
            .with_default_backup_dir(Some(backup.path().to_path_buf()))
            .build()
            .expect("runtime"),
    );
    service
        .pack(PackBundleAppRequest {
            installation: source_installation,
            manifest: sample_bundle_manifest(),
            output_path: Some(bundle_path.clone()),
            manifest_base_dir: None,
        })
        .expect("pack bundle");

    let applied = service
        .apply(ApplyBundleAppRequest {
            bundle_path,
            installation: target_installation,
            dry_run: false,
            backup_output_path: None,
            apply_mappings: BundleApplyMappingsValue::default(),
        })
        .expect("apply bundle with runtime backup dir");

    assert_eq!(
        applied.backup_path.as_deref().and_then(Path::parent),
        Some(backup.path())
    );
}

#[test]
fn bundle_service_addon_lock_shortcuts_use_runtime_addon_state_storage() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_bundle_fixture_installation(source.path(), false);
    let target_installation = create_bundle_fixture_installation(target.path(), false);
    let archive_path = source.path().join("WeakAuras.zip");
    let bundle_path = source.path().join("tracked.bundle.zip");
    let runtime = AppRuntime::builder()
        .with_addon_state_storage_kind(crate::core::addon::AddonStateStorageKind::Sidecar)
        .build()
        .expect("runtime");

    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );

    AddonService::with_runtime(runtime.clone())
        .install(InstallAddonAppRequest {
            installation: source_installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(source.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon into sidecar state");

    let mut manifest = sample_bundle_manifest();
    manifest.resources.addon_lock = true;

    BundleService::with_runtime(runtime.clone())
        .pack(PackBundleAppRequest {
            installation: source_installation,
            manifest,
            output_path: Some(bundle_path.clone()),
            manifest_base_dir: None,
        })
        .expect("pack bundle with sidecar-backed addon lock");

    let applied = BundleService::with_runtime(runtime.clone())
        .apply_addon_lock(ApplyBundleAddonLockAppRequest {
            bundle_path,
            installation: target_installation.clone(),
            backup_output_path: Some(target.path().join("addon-backups")),
            replace_existing: false,
        })
        .expect("apply embedded addon lock into sidecar state");

    assert!(applied.apply.verification.matches);

    let inventory = AddonService::with_runtime(runtime.clone())
        .list(ListAddonsRequest {
            installation: target_installation.clone(),
        })
        .expect("list sidecar-tracked addons");
    assert_eq!(inventory.tracked_package_count, 1);
    assert!(inventory.untracked_addons.is_empty());

    let target_domain_installation = target_installation
        .into_domain()
        .expect("resolved installation");
    let sidecar_paths = AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::Sidecar,
        &target_domain_installation,
    )
    .expect("sidecar state paths");
    let appdata_paths = AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::AppData,
        &target_domain_installation,
    )
    .expect("appdata state paths");

    assert!(sidecar_paths.registry_path.exists());
    assert!(sidecar_paths.lock_path.exists());
    assert!(!appdata_paths.registry_path.exists());
}

fn create_bundle_fixture_installation(
    root: &Path,
    with_content: bool,
) -> ResolvedInstallationValue {
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

fn sample_bundle_manifest() -> BundleManifestValue {
    BundleManifestValue {
        schema_version: 1,
        package: BundlePackageValue {
            id: "test-ui".to_string(),
            name: "Test UI".to_string(),
            created_by: "test".to_string(),
            description: None,
        },
        source: BundleSourceValue {
            flavor: WowFlavorValue::Retail,
            platform: None,
            exported_at: None,
            supported_targets: vec![WowFlavorValue::Retail],
        },
        resources: BundleResourcesValue {
            addons: vec!["WeakAuras".to_string()],
            wtf_common: false,
            wtf_characters: Vec::new(),
            fonts: false,
            interface_assets: Vec::new(),
            addon_lock: false,
            addon_indexes: Vec::new(),
        },
        mapping: BundleMappingRulesValue {
            character_mode: CharacterMappingModeValue::KeepOriginal,
            rewrite_profile_keys: false,
            rewrite_identity_strings: false,
            allow_cross_platform: true,
        },
        apply: BundleApplyDefaultsValue {
            create_backup: true,
            addons: ResourceApplyPolicyValue::Merge,
            wtf_common: ResourceApplyPolicyValue::Merge,
            wtf_characters: ResourceApplyPolicyValue::Merge,
            fonts: ResourceApplyPolicyValue::Merge,
            interface_assets: ResourceApplyPolicyValue::Merge,
        },
    }
}

fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

    for (name, contents) in entries {
        zip.start_file(name, options).expect("zip entry");
        zip.write_all(contents.as_bytes()).expect("zip write");
    }

    zip.finish().expect("finish archive");
}
