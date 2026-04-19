
use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::*;
use crate::core::app::{
    AppRuntime, BundleApplyDefaultsValue, BundleApplyMappingsValue, BundleManifestValue,
    BundleMappingRulesValue, BundlePackageValue, BundleResourcesValue, BundleSourceValue,
    CharacterMappingModeValue, HelperStrategyValue, ResolvedInstallationValue,
    ResourceApplyPolicyValue, WowFlavorValue,
};
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
        AppRuntime::new().with_default_bundle_output_dir(Some(output.path().to_path_buf())),
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
        AppRuntime::new().with_default_backup_dir(Some(backup.path().to_path_buf())),
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
