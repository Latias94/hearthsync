use std::cell::Cell;
use std::fs;
use std::io::Write;

use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use super::{
    AnalyzeExternalPackageRequest, ApplyExternalPackageRequest, BundleAddonLockApplyRequest,
    BundleApplyMappings, CreateExternalPackageBundleRequest, PackBundleRequest,
    PlanExternalPackageApplyRequest, UnpackBundleRequest, analyze_external_package,
    analyze_external_package_task, apply_bundle_addon_lock, apply_external_package,
    apply_external_package_task, create_external_package_bundle, inspect_bundle, pack_bundle,
    plan_bundle_addon_lock, plan_bundle_apply, plan_external_package_apply,
    plan_external_package_apply_task, unpack_bundle, unpack_bundle_task,
};
use crate::core::addon::lock::plan_addon_lock_sync;
use crate::core::addon::{InstallAddonRequest, install_addon};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};
use crate::core::task::{CancellationToken, NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

#[test]
fn pack_bundle_writes_normalized_layout() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path(), true);
    let bundle_path = temp.path().join("bundle.zip");

    let bundle = pack_bundle(PackBundleRequest {
        installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    assert_eq!(bundle.archive_path, bundle_path);

    let file = fs::File::open(bundle.archive_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");

    assert!(archive.by_name("manifest.toml").is_ok());
    assert!(archive.by_name("addons/WeakAuras/WeakAuras.toc").is_ok());
    assert!(archive.by_name("addons/WeakAuras/WeakAuras.lua").is_ok());
    assert!(archive.by_name("wtf/common/Config.wtf").is_ok());
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt")
            .is_ok()
    );
    assert!(archive.by_name("fonts/FRIZQT__.ttf").is_ok());
    assert!(archive.by_name("interface/SharedXML/texture.blp").is_ok());
}

#[test]
fn analyze_external_package_zip_normalizes_wrapped_ui_layout() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path.clone(),
    })
    .expect("analyze external package");

    assert_eq!(analysis.source_path, package_path);
    assert_eq!(
        analysis.source_kind,
        super::ExternalPackageSourceKind::ZipArchive
    );
    assert_eq!(analysis.package_id, "author-ui-pack");
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert!(analysis.resources.wtf_common);
    assert!(analysis.resources.fonts);
    assert_eq!(
        analysis.resources.interface_assets,
        vec!["SharedXML".to_string()]
    );
    assert_eq!(analysis.resources.wtf_characters.len(), 1);
    assert_eq!(
        analysis.resources.wtf_characters[0].source_account,
        Some("ACCOUNT".to_string())
    );
    assert_eq!(analysis.summary.total_files, 10);
    assert_eq!(analysis.summary.normalized_files, 9);
    assert_eq!(analysis.summary.ignored_files, 1);
    assert_eq!(analysis.summary.warning_count, 0);
    assert!(analysis.summary.warning_groups.is_empty());
    assert!(analysis.warnings.is_empty());

    let normalized_paths = analysis
        .entries
        .iter()
        .map(|entry| entry.normalized_path.as_str())
        .collect::<Vec<_>>();
    assert!(normalized_paths.contains(&"addons/WeakAuras/WeakAuras.toc"));
    assert!(normalized_paths.contains(&"wtf/common/Config.wtf"));
    assert!(normalized_paths.contains(&"wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"));
    assert!(normalized_paths.contains(&"wtf/common/accounts/ACCOUNT/bindings-cache.wtf"));
    assert!(normalized_paths.contains(&"wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt"));
    assert!(
        normalized_paths
            .contains(&"wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua")
    );
    assert!(normalized_paths.contains(&"fonts/FRIZQT__.ttf"));
    assert!(normalized_paths.contains(&"interface/SharedXML/texture.blp"));
}

#[test]
fn analyze_external_package_directory_fixture_normalizes_wrapped_ui_layout() {
    let package_root = external_package_fixture_root();

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root.clone(),
    })
    .expect("analyze external package directory fixture");

    assert_eq!(analysis.source_path, package_root);
    assert_eq!(
        analysis.source_kind,
        super::ExternalPackageSourceKind::Directory
    );
    assert_eq!(analysis.package_id, "external_package_author_ui_wrapped");
    assert_eq!(
        analysis.package_name,
        "external_package_author_ui_wrapped".to_string()
    );
    assert_eq!(analysis.summary.total_files, 10);
    assert_eq!(analysis.summary.normalized_files, 9);
    assert_eq!(analysis.summary.ignored_files, 1);
    assert_eq!(analysis.summary.warning_count, 0);
    assert!(analysis.summary.warning_groups.is_empty());
    assert!(analysis.warnings.is_empty());
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert!(analysis.resources.wtf_common);
    assert_eq!(analysis.resources.wtf_characters.len(), 1);
    assert!(analysis.resources.fonts);
    assert_eq!(
        analysis.resources.interface_assets,
        vec!["SharedXML".to_string()]
    );
}

#[test]
fn analyze_external_package_directory_dirty_fixture_reports_warnings_and_keeps_supported_entries() {
    let package_root = external_package_dirty_fixture_root();

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root.clone(),
    })
    .expect("analyze external package dirty fixture");

    assert_eq!(analysis.source_path, package_root);
    assert_eq!(
        analysis.source_kind,
        super::ExternalPackageSourceKind::Directory
    );
    assert_eq!(analysis.package_id, "external_package_dirty_mixed_case");
    assert_eq!(analysis.summary.total_files, 8);
    assert_eq!(analysis.summary.normalized_files, 7);
    assert_eq!(analysis.summary.ignored_files, 1);
    assert_eq!(analysis.summary.warning_count, 1);
    assert_eq!(analysis.summary.addon_warning_count, 1);
    assert_eq!(analysis.summary.wtf_warning_count, 0);
    assert_eq!(
        analysis.summary.warning_groups,
        vec![super::ExternalPackageWarningGroup {
            category: super::ExternalPackageWarningCategory::Addon,
            code: super::ExternalPackageWarningCode::AddonRootNotDetected,
            count: 1,
        }]
    );
    assert_eq!(analysis.resources.addons, vec!["Questie".to_string()]);
    assert!(analysis.resources.wtf_common);
    assert_eq!(analysis.resources.wtf_characters.len(), 1);
    assert_eq!(
        analysis.resources.wtf_characters[0].source_account,
        Some("ACC1".to_string())
    );
    assert!(analysis.resources.fonts);
    assert_eq!(
        analysis.resources.interface_assets,
        vec!["FrameXML".to_string()]
    );
    assert_eq!(analysis.warnings.len(), 1);
    assert!(analysis.warnings.iter().any(|warning| {
        warning.code == super::ExternalPackageWarningCode::AddonRootNotDetected
            && warning.message.contains("no addon root was detected")
            && warning.source_path.contains("BrokenAddon/README.txt")
    }));

    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/root/SavedVariables/Broken.lua"
            && entry.wtf_scope == Some(super::WtfScope::RootSavedVariables)
    }));
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/accounts/ACC1/config-cache.wtf"
            && entry.wtf_scope == Some(super::WtfScope::CacheLike)
    }));
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path
            == "wtf/characters/ACC1/Illidan/Targetone/SavedVariables/MeetingStone.lua"
            && entry.wtf_scope == Some(super::WtfScope::CharacterSavedVariables)
    }));
}

#[test]
fn analyze_external_package_zip_dirty_fixture_matches_directory_behavior() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("dirty-author-pack.zip");
    create_archive_from_directory(&external_package_dirty_fixture_root(), &package_path);

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path.clone(),
    })
    .expect("analyze dirty external package zip");

    assert_eq!(analysis.source_path, package_path);
    assert_eq!(
        analysis.source_kind,
        super::ExternalPackageSourceKind::ZipArchive
    );
    assert_eq!(analysis.package_id, "dirty-author-pack");
    assert_eq!(analysis.summary.total_files, 8);
    assert_eq!(analysis.summary.normalized_files, 7);
    assert_eq!(analysis.summary.ignored_files, 1);
    assert_eq!(analysis.summary.warning_count, 1);
    assert_eq!(
        analysis.summary.warning_groups,
        vec![super::ExternalPackageWarningGroup {
            category: super::ExternalPackageWarningCategory::Addon,
            code: super::ExternalPackageWarningCode::AddonRootNotDetected,
            count: 1,
        }]
    );
    assert_eq!(analysis.resources.addons, vec!["Questie".to_string()]);
    assert_eq!(analysis.warnings.len(), 1);
}

#[test]
fn analyze_external_package_directory_accepts_variant_toc_names() {
    let temp = tempdir().expect("temp dir");
    let package_root = temp.path().join("AuthorUI");
    let addon_root = package_root
        .join("Interface")
        .join("AddOns")
        .join("DBM-Core");

    fs::create_dir_all(&addon_root).expect("addon dir");
    fs::write(
        addon_root.join("DBM-Core_Mainline.toc"),
        "## Interface: 110000\n## Title: DBM Core\n",
    )
    .expect("toc");
    fs::write(addon_root.join("Core.lua"), "print('dbm')").expect("lua");

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root,
    })
    .expect("analyze external package with variant toc");

    assert_eq!(analysis.resources.addons, vec!["DBM-Core".to_string()]);
    assert_eq!(analysis.summary.warning_count, 0);
    assert!(analysis.warnings.is_empty());
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "addons/DBM-Core/DBM-Core_Mainline.toc"
            && entry.group == super::ApplyGroup::Addons
    }));
}

#[test]
fn analyze_external_package_conflict_fixture_exposes_duplicate_normalized_paths() {
    let package_root = external_package_conflict_fixture_root();

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root,
    })
    .expect("analyze external package conflict fixture");

    assert_eq!(analysis.summary.total_files, 2);
    assert_eq!(analysis.summary.normalized_files, 2);
    assert_eq!(analysis.summary.ignored_files, 0);
    assert_eq!(analysis.summary.warning_count, 0);
    assert!(analysis.summary.warning_groups.is_empty());
    assert!(analysis.warnings.is_empty());
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);

    let duplicate_count = analysis
        .entries
        .iter()
        .filter(|entry| entry.normalized_path == "addons/WeakAuras/WeakAuras.toc")
        .count();
    assert_eq!(duplicate_count, 2);
}

#[test]
fn create_external_package_bundle_rejects_duplicate_normalized_paths_from_directory_fixture() {
    let error =
        create_external_package_bundle(sample_external_package_request_with_apply_defaults(
            external_package_conflict_fixture_root(),
            None,
        ))
        .expect_err("duplicate normalized paths should fail");

    let message = error.to_string();
    assert!(message.contains("normalizes multiple files onto the same target path"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
}

#[test]
fn create_external_package_bundle_rejects_duplicate_normalized_paths_from_zip_fixture() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("conflicting-author-pack.zip");
    create_archive_from_directory(&external_package_conflict_fixture_root(), &package_path);

    let error = create_external_package_bundle(
        sample_external_package_request_with_apply_defaults(package_path, None),
    )
    .expect_err("duplicate normalized paths in zip should fail");

    let message = error.to_string();
    assert!(message.contains("normalizes multiple files onto the same target path"));
    assert!(message.contains("addons/WeakAuras/WeakAuras.toc"));
}

#[test]
fn create_external_package_bundle_rejects_case_insensitive_target_path_collisions_from_zip_fixture()
{
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("case-collision-author-pack.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("Fonts/FRIZQT__.ttf", "font-a"),
            ("fonts/frizqt__.ttf", "font-b"),
        ],
    );

    let error = create_external_package_bundle(
        sample_external_package_request_with_apply_defaults(package_path, None),
    )
    .expect_err("case-insensitive normalized path collisions should fail");

    let message = error.to_string();
    assert!(message.contains("case-insensitive target path collisions"));
    assert!(message.contains("fonts/FRIZQT__.ttf"));
    assert!(message.contains("fonts/frizqt__.ttf"));
}

#[test]
fn analyze_external_package_rejects_zip_with_parent_directory_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-parent.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("../evil.txt", "evil"),
            ("AuthorUI/WTF/Config.wtf", "SET locale enUS"),
        ],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("parent directory zip entry should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_zip_with_backslash_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-backslash.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("AuthorUI\\WTF\\Config.wtf", "SET locale enUS")],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("backslash zip entry should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_zip_with_empty_path_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-empty-segment.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("AuthorUI//WTF/Config.wtf", "SET locale enUS")],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("empty path segment zip entry should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_non_archive_file_with_clear_error() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("not-a-zip.bin");
    fs::write(&package_path, "plain text").expect("plain file");

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path.clone(),
    })
    .expect_err("plain file should not be treated as zip");

    let message = error.to_string();
    assert!(message.contains("not a valid zip archive"));
    assert!(message.contains(&package_path.display().to_string()));
}

#[test]
fn create_external_package_bundle_rejects_zip_with_only_directory_entries() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("directory-only.zip");
    create_archive_with_raw_directories(
        &package_path,
        &[
            "AuthorUI/",
            "AuthorUI/Interface/",
            "AuthorUI/Interface/AddOns/",
            "AuthorUI/WTF/",
        ],
    );

    let error = create_external_package_bundle(
        sample_external_package_request_with_apply_defaults(package_path, None),
    )
    .expect_err("directory-only zip should not build a bundle");

    let message = error.to_string();
    assert!(message.contains("resources must include at least one addon"));
}

#[test]
fn analyze_external_package_directory_detects_direct_addons_and_root_savedvariables() {
    let temp = tempdir().expect("temp dir");
    let package_root = temp.path().join("AuthorPack");

    fs::create_dir_all(package_root.join("WeakAuras")).expect("addon dir");
    fs::write(
        package_root.join("WeakAuras").join("WeakAuras.toc"),
        "## Interface: 110000",
    )
    .expect("addon toc");
    fs::create_dir_all(
        package_root
            .join("WTF")
            .join("Account")
            .join("SavedVariables"),
    )
    .expect("unsupported wtf dir");
    fs::write(
        package_root
            .join("WTF")
            .join("Account")
            .join("SavedVariables")
            .join("Broken.lua"),
        "Broken = true",
    )
    .expect("unsupported wtf file");
    fs::create_dir_all(package_root.join("Fonts")).expect("fonts dir");
    fs::write(package_root.join("Fonts").join("FRIZQT__.ttf"), "font").expect("font");

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root,
    })
    .expect("analyze external package directory");

    assert_eq!(
        analysis.source_kind,
        super::ExternalPackageSourceKind::Directory
    );
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert!(analysis.resources.fonts);
    assert!(analysis.resources.wtf_common);
    assert_eq!(analysis.summary.total_files, 3);
    assert_eq!(analysis.summary.normalized_files, 3);
    assert_eq!(analysis.summary.ignored_files, 0);
    assert_eq!(analysis.summary.warning_count, 0);
    assert_eq!(analysis.summary.wtf_warning_count, 0);
    assert!(analysis.summary.warning_groups.is_empty());
    assert!(analysis.warnings.is_empty());
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/root/SavedVariables/Broken.lua"
            && entry.wtf_scope == Some(super::WtfScope::RootSavedVariables)
    }));
}

#[test]
fn create_external_package_bundle_produces_reusable_first_party_bundle() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let prepared = create_external_package_bundle(CreateExternalPackageBundleRequest {
        source_path: package_path,
        source_flavor: WowFlavor::Retail,
        source_platform: Some(HostPlatform::Windows),
        supported_targets: vec![WowFlavor::Retail],
        output_path: None,
        package_id: None,
        package_name: None,
        created_by: None,
        description: None,
        apply_defaults: None,
    })
    .expect("create external package bundle");

    assert!(prepared.bundle.archive_path.exists());
    assert_eq!(prepared.manifest.package.id, "author-ui-pack");
    assert_eq!(
        prepared.manifest.resources.addons,
        vec!["WeakAuras".to_string()]
    );
    assert_eq!(
        prepared.manifest.mapping.character_mode,
        CharacterMappingMode::Prompt
    );

    let inspection = inspect_bundle(&prepared.bundle.archive_path).expect("inspect bundle");
    assert_eq!(inspection.manifest.package.id, "author-ui-pack");
    assert_eq!(inspection.entries.addons, 2);
    assert_eq!(inspection.entries.wtf_common, 3);
    assert_eq!(inspection.entries.wtf_characters, 2);
    assert_eq!(inspection.entries.fonts, 1);
    assert_eq!(inspection.entries.interface_assets, 1);
}

#[test]
fn external_package_bundle_can_reuse_plan_and_unpack_pipeline() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let prepared = create_external_package_bundle(CreateExternalPackageBundleRequest {
        source_path: package_path,
        source_flavor: WowFlavor::Retail,
        source_platform: Some(HostPlatform::Windows),
        supported_targets: vec![WowFlavor::Retail],
        output_path: None,
        package_id: Some("author-ui-import".to_string()),
        package_name: Some("Author UI Import".to_string()),
        created_by: Some("hearthsync-test".to_string()),
        description: None,
        apply_defaults: None,
    })
    .expect("create external package bundle");
    let target_installation = create_fixture_installation(target.path(), false);
    let apply_mappings = BundleApplyMappings {
        target_account: Some("ACCOUNT".to_string()),
        target_server: Some("Illidan".to_string()),
        target_character: Some("Examplemage".to_string()),
        ..BundleApplyMappings::default()
    };

    let plan = plan_bundle_apply(
        &prepared.bundle.archive_path,
        &target_installation,
        &apply_mappings,
    )
    .expect("plan external package bundle");
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == super::ApplyGroup::Addons)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == super::ApplyGroup::WtfCommon)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == super::ApplyGroup::WtfCharacters)
    );

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path: prepared.bundle.archive_path.clone(),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings,
    })
    .expect("apply external package bundle");

    assert!(result.written_files > 0);
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(target_installation.wtf_dir.join("Config.wtf").exists());
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("SavedVariables")
            .join("Pawn.lua")
            .exists()
    );
    assert!(target_installation.fonts_dir.join("FRIZQT__.ttf").exists());
    assert!(
        target_installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );
}

#[test]
fn plan_external_package_apply_wraps_normalization_and_bundle_planning() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation = create_fixture_installation(target.path(), false);
    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: CreateExternalPackageBundleRequest {
            source_path: package_path,
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
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package apply");

    assert_eq!(
        plan.analysis.resources.addons,
        vec!["WeakAuras".to_string()]
    );
    assert_eq!(plan.target_flavor_root, target_installation.flavor_root);
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == super::ApplyGroup::Addons)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == super::ApplyGroup::WtfCommon)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == super::ApplyGroup::WtfCharacters)
    );
}

#[test]
fn analyze_external_package_task_reports_progress() {
    let package_root = external_package_dirty_fixture_root();
    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;

    let analysis = analyze_external_package_task(
        AnalyzeExternalPackageRequest {
            source_path: package_root,
        },
        &cancellation,
        &mut progress,
    )
    .expect("analyze external package task");

    assert_eq!(analysis.summary.warning_count, 1);
    assert_eq!(
        progress
            .events()
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
fn plan_external_package_apply_task_reports_progress() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation = create_fixture_installation(target.path(), false);
    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let plan = plan_external_package_apply_task(
        PlanExternalPackageApplyRequest {
            external_package: CreateExternalPackageBundleRequest {
                source_path: package_path,
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
            apply_mappings: BundleApplyMappings {
                target_account: Some("ACCOUNT".to_string()),
                target_server: Some("Illidan".to_string()),
                target_character: Some("Examplemage".to_string()),
                ..BundleApplyMappings::default()
            },
        },
        &cancellation,
        &mut progress,
    )
    .expect("plan external package task");

    assert!(!plan.operations.is_empty());
    assert_eq!(
        progress
            .events()
            .iter()
            .map(|event| (event.task, event.phase))
            .collect::<Vec<_>>(),
        vec![
            (TaskKind::ExternalPackagePlan, TaskPhase::Preparing),
            (TaskKind::ExternalPackagePlan, TaskPhase::Planning),
            (TaskKind::ExternalPackagePlan, TaskPhase::Completed),
        ]
    );
}

#[test]
fn apply_external_package_task_wraps_normalization_and_apply_progress() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation = create_fixture_installation(target.path(), false);
    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = apply_external_package_task(
        ApplyExternalPackageRequest {
            external_package: CreateExternalPackageBundleRequest {
                source_path: package_path,
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
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings {
                target_account: Some("ACCOUNT".to_string()),
                target_server: Some("Illidan".to_string()),
                target_character: Some("Examplemage".to_string()),
                ..BundleApplyMappings::default()
            },
        },
        &cancellation,
        &mut progress,
    )
    .expect("apply external package task");

    assert!(result.written_files > 0);
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(target_installation.wtf_dir.join("Config.wtf").exists());
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.message.contains("Normalizing external package"))
    );
    assert!(
        progress
            .events()
            .iter()
            .all(|event| event.task == TaskKind::ExternalPackageApply)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Preparing)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Planning)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Executing)
    );
    assert!(
        progress
            .events()
            .iter()
            .any(|event| event.phase == TaskPhase::Completed)
    );
    assert!(progress.events().iter().any(|event| {
        event.phase == TaskPhase::Executing && event.message.contains("operation 1/")
    }));

    let second_target = tempdir().expect("second target");
    let direct_result = apply_external_package(ApplyExternalPackageRequest {
        external_package: CreateExternalPackageBundleRequest {
            source_path: source.path().join("author-ui-pack.zip"),
            source_flavor: WowFlavor::Retail,
            source_platform: Some(HostPlatform::Windows),
            supported_targets: vec![WowFlavor::Retail],
            output_path: None,
            package_id: Some("author-ui-direct".to_string()),
            package_name: Some("Author UI Direct".to_string()),
            created_by: None,
            description: None,
            apply_defaults: None,
        },
        installation: create_fixture_installation(second_target.path(), false),
        dry_run: true,
        backup_output_path: None,
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("apply external package directly");
    assert!(direct_result.dry_run);
}

#[test]
fn plan_external_package_apply_supports_windows_package_to_macos_target_with_policy_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: sample_external_package_request_with_apply_defaults(
            package_path,
            Some(ApplyDefaults {
                create_backup: false,
                addons: ResourceApplyPolicy::Mirror,
                wtf_common: ResourceApplyPolicy::Share,
                wtf_characters: ResourceApplyPolicy::ReplaceSelected,
                fonts: ResourceApplyPolicy::Preserve,
                interface_assets: ResourceApplyPolicy::ReplaceSelected,
            }),
        ),
        installation: target_installation.clone(),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package apply");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert_eq!(plan.manifest.source.platform, Some(HostPlatform::Windows));
    assert!(plan.manifest.mapping.allow_cross_platform);
    assert!(!plan.manifest.apply.create_backup);
    assert_eq!(
        plan.group_policies.addons.policy,
        ResourceApplyPolicy::Mirror
    );
    assert_eq!(
        plan.group_policies.wtf_common.policy,
        ResourceApplyPolicy::Share
    );
    assert_eq!(
        plan.group_policies.wtf_characters.policy,
        ResourceApplyPolicy::ReplaceSelected
    );
    assert_eq!(
        plan.group_policies.fonts.policy,
        ResourceApplyPolicy::Preserve
    );
    assert_eq!(
        plan.group_policies.interface_assets.policy,
        ResourceApplyPolicy::ReplaceSelected
    );
    assert_eq!(plan.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(plan.summary.paths_to_remove, 3);
    assert_eq!(plan.summary.files_to_add, 7);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.summary.files_to_preserve, 2);

    assert!(plan.operations.iter().any(|operation| {
        operation.action == super::ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == super::ApplyAction::Remove
            && operation.destination == target_installation.interface_dir.join("SharedXML")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == super::ApplyAction::Remove
            && operation.destination
                == target_installation
                    .wtf_dir
                    .join("Account")
                    .join("ACCOUNT")
                    .join("Illidan")
                    .join("Examplemage")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/Config.wtf"
            && operation.action == super::ApplyAction::Preserve
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "fonts/FRIZQT__.ttf"
            && operation.action == super::ApplyAction::Preserve
    }));
}

#[test]
fn apply_external_package_respects_policy_overrides_on_macos_target() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let result = apply_external_package(ApplyExternalPackageRequest {
        external_package: sample_external_package_request_with_apply_defaults(
            package_path,
            Some(ApplyDefaults {
                create_backup: false,
                addons: ResourceApplyPolicy::Mirror,
                wtf_common: ResourceApplyPolicy::Share,
                wtf_characters: ResourceApplyPolicy::ReplaceSelected,
                fonts: ResourceApplyPolicy::Preserve,
                interface_assets: ResourceApplyPolicy::ReplaceSelected,
            }),
        ),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("apply external package");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert!(!result.dry_run);
    assert_eq!(result.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(result.written_files, 7);
    assert_eq!(result.rewritten_files, 0);
    assert!(result.backup_path.is_none());
    assert_eq!(result.plan_summary.paths_to_remove, 3);
    assert_eq!(result.plan_summary.files_to_add, 7);
    assert_eq!(result.plan_summary.files_to_preserve, 2);

    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(target_installation.wtf_dir.join("Config.wtf")).expect("target config"),
        "SET locale zhCN"
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("bindings-cache.wtf")
            .exists()
    );
    assert!(
        !target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("StaleCharacter.txt")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("SavedVariables")
            .join("Pawn.lua")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(target_installation.fonts_dir.join("FRIZQT__.ttf")).expect("font"),
        "mac-font"
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("old.blp")
            .exists()
    );
    assert_eq!(
        fs::read_to_string(
            target_installation
                .interface_dir
                .join("SharedXML")
                .join("texture.blp")
        )
        .expect("texture")
        .trim_end(),
        "fixture-texture"
    );
}

#[test]
fn unpack_bundle_task_reports_progress_for_dry_run() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation,
            dry_run: true,
            backup_output_path: None,
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect("unpack bundle dry run task");

    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases,
        vec![
            (TaskKind::BundleApply, TaskPhase::Preparing),
            (TaskKind::BundleApply, TaskPhase::Planning),
            (TaskKind::BundleApply, TaskPhase::Completed),
        ]
    );
    assert!(result.dry_run);
}

#[test]
fn unpack_bundle_task_honors_cancellation_before_execution() {
    struct CancelOnSecondCheck {
        checks: Cell<usize>,
    }

    impl CancellationToken for CancelOnSecondCheck {
        fn is_cancelled(&self) -> bool {
            let next = self.checks.get() + 1;
            self.checks.set(next);
            next >= 2
        }
    }

    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = CancelOnSecondCheck {
        checks: Cell::new(0),
    };
    let error = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("bundle task should cancel before execution");

    assert!(matches!(error, crate::core::error::AppError::Cancelled(_)));
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn unpack_bundle_task_reports_operation_progress_during_execution() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = NeverCancel;
    let result = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation,
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect("bundle task should complete");

    let executing_messages = progress
        .events()
        .iter()
        .filter(|event| event.task == TaskKind::BundleApply && event.phase == TaskPhase::Executing)
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();

    assert!(result.written_files > 0);
    assert!(executing_messages.len() > 1);
    assert!(executing_messages.iter().any(|message| {
        message.contains("operation 1/") && message.contains("Executing bundle operation")
    }));
}

#[test]
fn unpack_bundle_task_honors_cancellation_during_execution_loop() {
    struct CancelOnFifthCheck {
        checks: Cell<usize>,
    }

    impl CancellationToken for CancelOnFifthCheck {
        fn is_cancelled(&self) -> bool {
            let next = self.checks.get() + 1;
            self.checks.set(next);
            next >= 5
        }
    }

    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let mut progress = VecTaskProgressSink::default();
    let cancellation = CancelOnFifthCheck {
        checks: Cell::new(0),
    };
    let error = unpack_bundle_task(
        UnpackBundleRequest {
            bundle_path,
            installation: target_installation.clone(),
            dry_run: false,
            backup_output_path: Some(target.path().join("backups")),
            apply_mappings: BundleApplyMappings::default(),
        },
        &cancellation,
        &mut progress,
    )
    .expect_err("bundle task should cancel during execution loop");

    assert!(matches!(error, crate::core::error::AppError::Cancelled(_)));
    assert!(progress.events().iter().any(|event| {
        event.task == TaskKind::BundleApply
            && event.phase == TaskPhase::Executing
            && event.message.contains("operation 1/")
    }));
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn unpack_bundle_restores_files_and_creates_backup() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path: bundle_path.clone(),
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(result.bundle_path, bundle_path);
    assert!(result.written_files > 0);
    assert!(
        result
            .backup_path
            .as_ref()
            .is_some_and(|path| path.exists())
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(target_installation.wtf_dir.join("Config.wtf").exists());
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("AddOns.txt")
            .exists()
    );
    assert!(target_installation.fonts_dir.join("FRIZQT__.ttf").exists());
    assert!(
        target_installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );

    let inspection = inspect_bundle(&result.bundle_path).expect("inspect bundle");
    assert_eq!(inspection.entries.addons, 2);
    assert_eq!(inspection.entries.fonts, 1);
}

#[test]
fn plan_bundle_apply_discovers_local_accounts_and_selected_accounts() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables"),
    )
    .expect("account a");
    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables"),
    )
    .expect("account b");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            selected_accounts: vec!["ACC_A".to_string()],
            ..BundleApplyMappings::default()
        },
    )
    .expect("plan bundle");

    assert_eq!(plan.discovered_accounts.len(), 2);
    assert_eq!(plan.selected_target_accounts, vec!["ACC_A".to_string()]);
    assert!(plan.summary.files_to_add > 0);
    assert!(plan.operations.iter().any(|item| {
        item.group == super::ApplyGroup::WtfCommon
            && item.target_account.as_deref() == Some("ACC_A")
    }));
}

#[test]
fn keep_original_character_mode_ignores_target_identity_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::KeepOriginal;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            target_account: Some("TARGETACC".to_string()),
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            ..BundleApplyMappings::default()
        },
    )
    .expect("plan bundle");

    assert_eq!(plan.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(plan.character_mappings.len(), 1);
    assert_eq!(plan.character_mappings[0].target_account, "ACCOUNT");
    assert_eq!(plan.character_mappings[0].target_server, "Illidan");
    assert_eq!(plan.character_mappings[0].target_character, "Examplemage");
}

#[test]
fn explicit_character_mode_requires_resolved_target_identity() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect_err("explicit mode should require a resolved target identity");

    assert!(
        error
            .to_string()
            .contains("explicit character mode requires a fully resolved target identity")
    );
    assert!(error.to_string().contains("--mapping-file"));
}

#[test]
fn prompt_character_mode_requires_resolved_target_identity() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Prompt;
    manifest.resources.wtf_characters[0].target_hint = Some("Map to your main".to_string());

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect_err("prompt mode should require caller-provided mappings");

    assert!(
        error
            .to_string()
            .contains("current CLI does not prompt automatically")
    );
    assert!(error.to_string().contains("Map to your main"));
}

#[test]
fn multi_character_explicit_mode_rejects_global_target_identity_overrides() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;
    manifest.resources.wtf_characters.push(CharacterResource {
        source_account: Some("ACCOUNT".to_string()),
        source_server: "Illidan".to_string(),
        source_character: "Altmage".to_string(),
        target_hint: None,
    });
    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Altmage"),
    )
    .expect("alt character");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Altmage")
            .join("AddOns.txt"),
        "Altmage",
    )
    .expect("alt addons");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let error = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings {
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            ..BundleApplyMappings::default()
        },
    )
    .expect_err("multi-character explicit mode should reject global target identity");

    assert!(error.to_string().contains("exactly one character"));
    assert!(error.to_string().contains("--mapping-file"));
}

#[test]
fn bundle_apply_plan_does_not_expose_execution_only_fields() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest_with_rewrite(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    let operations = serde_json::to_value(&plan)
        .expect("serialize plan")
        .get("operations")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("operations array");

    assert!(!operations.is_empty());
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("staged_path").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrites").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrite_count").is_none())
    );
    assert!(
        operations
            .iter()
            .all(|operation| operation.get("rewrite_applied").is_none())
    );
}

#[test]
fn analyze_external_package_serializes_warning_groups_for_machine_consumers() {
    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: external_package_dirty_fixture_root(),
    })
    .expect("analyze dirty external package");

    let summary = serde_json::to_value(&analysis)
        .expect("serialize analysis")
        .get("summary")
        .cloned()
        .expect("summary field");
    let warning_groups = summary
        .get("warning_groups")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("warning_groups array");

    assert_eq!(
        warning_groups,
        vec![serde_json::json!({
            "category": "addon",
            "code": "addon_root_not_detected",
            "count": 1
        }),]
    );
}

#[test]
fn external_package_warning_code_serialization_matches_display_codes() {
    let codes = [
        super::ExternalPackageWarningCode::AddonRootNotDetected,
        super::ExternalPackageWarningCode::UnsupportedWtfLayout,
        super::ExternalPackageWarningCode::UnsupportedWtfRootSavedVariables,
        super::ExternalPackageWarningCode::WtfAccountPathWithoutFile,
        super::ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
        super::ExternalPackageWarningCode::UnsupportedWtfNestedAccountLayout,
    ];

    for code in codes {
        let serialized = serde_json::to_value(code)
            .expect("serialize warning code")
            .as_str()
            .map(str::to_string)
            .expect("warning code string");
        assert_eq!(serialized, code.as_str(), "unexpected code serialization");
    }
}

#[test]
fn bundle_apply_plan_uses_explicit_resource_group_order() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    let mut groups = Vec::new();
    for operation in plan
        .operations
        .iter()
        .filter(|operation| operation.action == super::ApplyAction::Add)
    {
        if groups.last().copied() != Some(operation.group) {
            groups.push(operation.group);
        }
    }

    assert_eq!(
        groups,
        vec![
            super::ApplyGroup::Addons,
            super::ApplyGroup::InterfaceAssets,
            super::ApplyGroup::Fonts,
            super::ApplyGroup::WtfCommon,
            super::ApplyGroup::WtfCharacters,
        ]
    );
}

#[test]
fn plan_bundle_apply_classifies_wtf_scopes_and_account_root_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let source_account_dir = source_installation.wtf_dir.join("Account").join("ACCOUNT");
    let source_character_dir = source_account_dir.join("Illidan").join("Examplemage");

    fs::write(
        source_account_dir.join("account-settings.wtf"),
        "account root",
    )
    .expect("account root file");
    fs::write(source_account_dir.join("config-cache.wtf"), "account cache")
        .expect("account cache file");
    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables"),
    )
    .expect("root saved variables");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
        "DetailsDB = {}",
    )
    .expect("root saved variable");
    fs::create_dir_all(source_character_dir.join("SavedVariables"))
        .expect("character saved variables");
    fs::write(
        source_character_dir.join("SavedVariables").join("Pawn.lua"),
        "PawnDB = {}",
    )
    .expect("character saved variable");
    fs::write(
        source_character_dir.join("config-cache.wtf"),
        "character cache",
    )
    .expect("character cache file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let file = fs::File::open(&bundle_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/account-settings.wtf")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/common/accounts/ACCOUNT/config-cache.wtf")
            .is_ok()
    );
    assert!(
        archive
            .by_name("wtf/common/root/SavedVariables/RootDetails.lua")
            .is_ok()
    );

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    let scope_for = |archive_name: &str| {
        plan.operations
            .iter()
            .find(|operation| operation.archive_name == archive_name)
            .and_then(|operation| operation.wtf_scope)
    };

    assert_eq!(
        scope_for("wtf/common/Config.wtf"),
        Some(super::WtfScope::GlobalConfig)
    );
    assert_eq!(
        scope_for("wtf/common/root/SavedVariables/RootDetails.lua"),
        Some(super::WtfScope::RootSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/account-settings.wtf"),
        Some(super::WtfScope::AccountRootFile)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"),
        Some(super::WtfScope::AccountSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua"),
        Some(super::WtfScope::CharacterSavedVariables)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/AddOns.txt"),
        Some(super::WtfScope::CharacterState)
    );
    assert_eq!(
        scope_for("wtf/common/accounts/ACCOUNT/config-cache.wtf"),
        Some(super::WtfScope::CacheLike)
    );
    assert_eq!(
        scope_for("wtf/characters/ACCOUNT/Illidan/Examplemage/config-cache.wtf"),
        Some(super::WtfScope::CacheLike)
    );
}

#[test]
fn plan_bundle_apply_skips_identical_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");

    assert_eq!(plan.summary.files_to_add, 0);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert!(plan.summary.files_to_skip > 0);
    assert_eq!(plan.summary.files_to_skip, plan.operations.len());
    assert_eq!(
        plan.group_policies.addons.policy,
        ResourceApplyPolicy::Merge
    );
    assert!(
        plan.operations
            .iter()
            .all(|operation| operation.action == super::ApplyAction::Skip)
    );
}

#[test]
fn unpack_bundle_applies_character_mapping_and_lua_rewrite() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest_with_rewrite();
    manifest.mapping.character_mode = CharacterMappingMode::Explicit;

    fs::create_dir_all(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables"),
    )
    .expect("root saved variables");
    fs::write(
        source_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
        r#"
DetailsDB = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["profiles"] = {
    ["Default.Illidan.Examplemage"] = {},
  },
}
"#,
    )
    .expect("root saved variable");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            target_account: Some("TARGETACC".to_string()),
            target_server: Some("Stormrage".to_string()),
            target_character: Some("Targetmage".to_string()),
            selected_accounts: Vec::new(),
            all_accounts: false,
            characters: Vec::new(),
        },
    })
    .expect("unpack bundle");

    assert_eq!(result.character_mappings.len(), 1);
    assert!(result.rewritten_files >= 2);
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("Stormrage")
            .join("Targetmage")
            .join("SavedVariables")
            .join("Pawn.lua")
            .exists()
    );

    let common_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("SavedVariables")
            .join("Details.lua"),
    )
    .expect("common lua");
    assert!(common_lua.contains("Targetmage - Stormrage"));
    assert!(common_lua.contains("Default.Stormrage.Targetmage"));

    let root_common_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join("RootDetails.lua"),
    )
    .expect("root common lua");
    assert!(root_common_lua.contains("Targetmage - Stormrage"));
    assert!(root_common_lua.contains("Default.Stormrage.Targetmage"));

    let character_lua = fs::read_to_string(
        target_installation
            .wtf_dir
            .join("Account")
            .join("TARGETACC")
            .join("Stormrage")
            .join("Targetmage")
            .join("SavedVariables")
            .join("Pawn.lua"),
    )
    .expect("character lua");
    assert!(character_lua.contains(r#""Targetmage""#));
    assert!(character_lua.contains(r#""Stormrage""#));

    let addon_lua = fs::read_to_string(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.lua"),
    )
    .expect("addon lua");
    assert!(addon_lua.contains("Examplemage - Illidan"));
    assert!(!addon_lua.contains("Targetmage - Stormrage"));
}

#[test]
fn unpack_bundle_replicates_common_wtf_to_selected_accounts() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables"),
    )
    .expect("account a");
    fs::create_dir_all(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables"),
    )
    .expect("account b");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings {
            selected_accounts: vec!["ACC_A".to_string(), "ACC_B".to_string()],
            ..BundleApplyMappings::default()
        },
    })
    .expect("unpack bundle");

    assert_eq!(
        result.selected_target_accounts,
        vec!["ACC_A".to_string(), "ACC_B".to_string()]
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_A")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACC_B")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
}

#[test]
fn unpack_bundle_rolls_back_when_apply_fails() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");

    fs::write(
        source_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
        "## Interface: 120000",
    )
    .expect("updated toc");
    fs::write(
        source_installation
            .addon_dir
            .join("WeakAuras")
            .join("Extra.lua"),
        "print('extra')",
    )
    .expect("extra addon file");
    fs::write(
        source_installation.wtf_dir.join("Config.wtf"),
        "SET locale zhCN",
    )
    .expect("updated config");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let original_toc = fs::read_to_string(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc"),
    )
    .expect("original toc");
    let original_config = fs::read_to_string(target_installation.wtf_dir.join("Config.wtf"))
        .expect("original config");

    let shared_xml = target_installation.interface_dir.join("SharedXML");
    fs::remove_dir_all(&shared_xml).expect("remove shared xml");
    fs::write(&shared_xml, "blocking file").expect("blocking file");

    let error = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect_err("unpack should fail");

    assert!(error.to_string().contains("rollback restored"));
    assert_eq!(
        fs::read_to_string(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("restored toc"),
        original_toc
    );
    assert_eq!(
        fs::read_to_string(target_installation.wtf_dir.join("Config.wtf"))
            .expect("restored config"),
        original_config
    );
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Extra.lua")
            .exists()
    );
    assert!(shared_xml.is_file());
    assert_eq!(
        fs::read_to_string(&shared_xml).expect("restored blocking file"),
        "blocking file"
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("texture.blp")
            .exists()
    );
}

#[test]
fn unpack_bundle_dry_run_does_not_write_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest: sample_manifest(),
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: true,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("dry run");

    assert!(result.dry_run);
    assert!(result.planned_files > 0);
    assert_eq!(result.written_files, 0);
    assert!(result.backup_path.is_none());
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn preserve_policy_plans_without_writing_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_common = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_characters = ResourceApplyPolicy::Preserve;
    manifest.apply.fonts = ResourceApplyPolicy::Preserve;
    manifest.apply.interface_assets = ResourceApplyPolicy::Preserve;

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.files_to_preserve > 0);
    assert_eq!(plan.summary.files_to_add, 0);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.summary.files_to_preserve, plan.operations.len());
    assert!(
        plan.operations
            .iter()
            .all(|operation| operation.action == super::ApplyAction::Preserve)
    );

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(result.written_files, 0);
    assert_eq!(result.plan_summary.files_to_preserve, result.planned_files);
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
    assert!(!target_installation.wtf_dir.join("Config.wtf").exists());
}

#[test]
fn share_policy_preserves_existing_target_files_and_adds_missing_files() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Preserve;
    manifest.apply.wtf_common = ResourceApplyPolicy::Share;
    manifest.apply.wtf_characters = ResourceApplyPolicy::Preserve;
    manifest.apply.fonts = ResourceApplyPolicy::Preserve;
    manifest.apply.interface_assets = ResourceApplyPolicy::Preserve;

    fs::write(
        target_installation.wtf_dir.join("Config.wtf"),
        "SET locale zhCN",
    )
    .expect("existing target config");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/Config.wtf"
            && operation.action == super::ApplyAction::Preserve
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"
            && operation.action == super::ApplyAction::Add
    }));

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert_eq!(
        fs::read_to_string(target_installation.wtf_dir.join("Config.wtf")).expect("target config"),
        "SET locale zhCN"
    );
    assert!(
        target_installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("SavedVariables")
            .join("Details.lua")
            .exists()
    );
    assert!(result.plan_summary.files_to_preserve >= 1);
    assert!(result.written_files >= 1);
}

#[test]
fn mirror_policy_removes_existing_addon_root_before_copy() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Mirror;

    fs::write(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.paths_to_remove >= 1);
    assert!(plan.operations.iter().any(|operation| {
        operation.action == super::ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));

    let result = unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert!(result.written_files > 0);
    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn sync_policy_alias_removes_existing_addon_root_before_copy() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), true);
    let target_installation = create_fixture_installation(target.path(), true);
    let bundle_path = source.path().join("bundle.zip");
    let mut manifest = sample_manifest();
    manifest.apply.addons = ResourceApplyPolicy::Sync;

    fs::write(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon file");

    pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: None,
    })
    .expect("pack bundle");

    let plan = plan_bundle_apply(
        &bundle_path,
        &target_installation,
        &BundleApplyMappings::default(),
    )
    .expect("plan bundle");
    assert!(plan.summary.paths_to_remove >= 1);
    assert!(plan.operations.iter().any(|operation| {
        operation.action == super::ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));

    unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    assert!(
        !target_installation
            .addon_dir
            .join("WeakAuras")
            .join("Stale.lua")
            .exists()
    );
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

#[test]
fn pack_bundle_embeds_addon_lock_and_indexes_as_sidecar_metadata() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let source_installation = create_fixture_installation(source.path(), false);
    let target_installation = create_fixture_installation(target.path(), false);
    let bundle_path = source.path().join("bundle.zip");
    let archive_path = source.path().join("WeakAuras.zip");
    let index_path = source.path().join("addon-index.toml");

    create_addon_archive(
        &archive_path,
        &[(
            "WeakAuras/WeakAuras.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    install_addon(InstallAddonRequest {
        installation: source_installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(source.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");
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
    .expect("index");

    let mut manifest = sample_manifest();
    manifest.resources.addons = Vec::new();
    manifest.resources.wtf_common = false;
    manifest.resources.wtf_characters = Vec::new();
    manifest.resources.fonts = false;
    manifest.resources.interface_assets = Vec::new();
    manifest.resources.addon_lock = true;
    manifest.resources.addon_indexes = vec!["addon-index.toml".to_string()];
    manifest.mapping.character_mode = CharacterMappingMode::KeepOriginal;
    manifest.apply.addons = ResourceApplyPolicy::Mirror;

    let bundle = pack_bundle(PackBundleRequest {
        installation: source_installation,
        manifest,
        output_path: Some(bundle_path.clone()),
        manifest_base_dir: Some(source.path().to_path_buf()),
    })
    .expect("pack bundle");

    let file = fs::File::open(&bundle.archive_path).expect("bundle file");
    let mut archive = ZipArchive::new(file).expect("zip archive");
    assert!(archive.by_name("metadata/addons/lock.toml").is_ok());
    assert!(archive.by_name("metadata/addons/sources.toml").is_ok());
    assert!(
        archive
            .by_name("metadata/addons/sources/addons-weakauras.zip")
            .is_ok()
    );
    assert!(
        archive
            .by_name("metadata/addons/indexes/addon-index.toml")
            .is_ok()
    );

    let inspection = inspect_bundle(&bundle.archive_path).expect("inspect bundle");
    assert_eq!(inspection.entries.metadata, 5);
    assert_eq!(
        inspection.manifest.apply.addons,
        ResourceApplyPolicy::Mirror
    );
    fs::remove_file(&archive_path).expect("remove original addon source");

    unpack_bundle(UnpackBundleRequest {
        bundle_path,
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: Some(target.path().join("backups")),
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("unpack bundle");

    let sidecar_root = target_installation
        .addon_dir
        .join(".hearthsync")
        .join("bundles")
        .join("test-ui");
    assert!(sidecar_root.join("addons").join("lock.toml").exists());
    assert!(
        sidecar_root
            .join("addons")
            .join("indexes")
            .join("addon-index.toml")
            .exists()
    );

    let sidecar_plan = plan_addon_lock_sync(
        &target_installation,
        Some(&sidecar_root.join("addons").join("lock.toml")),
    )
    .expect("sidecar addon plan");
    assert_eq!(sidecar_plan.install_count, 1);
    assert_eq!(sidecar_plan.blocked_count, 0);

    let addon_plan =
        plan_bundle_addon_lock(&bundle.archive_path, &target_installation).expect("addon plan");
    assert_eq!(addon_plan.plan.install_count, 1);
    assert_eq!(addon_plan.plan.update_count, 0);
    assert_eq!(addon_plan.plan.remove_count, 0);
    assert_eq!(addon_plan.plan.blocked_count, 0);

    let addon_apply = apply_bundle_addon_lock(BundleAddonLockApplyRequest {
        bundle_path: bundle.archive_path,
        installation: target_installation.clone(),
        backup_output_path: Some(target.path().join("addon-backups")),
        replace_existing: false,
    })
    .expect("addon apply");
    assert!(addon_apply.apply.verification.matches);
    assert!(
        target_installation
            .addon_dir
            .join("WeakAuras")
            .join("WeakAuras.toc")
            .exists()
    );
}

fn create_fixture_installation(
    root: &std::path::Path,
    with_content: bool,
) -> DetectedFlavorInstallation {
    create_fixture_installation_on_platform(root, with_content, HostPlatform::Windows)
}

fn create_fixture_installation_on_platform(
    root: &std::path::Path,
    with_content: bool,
    platform: HostPlatform,
) -> DetectedFlavorInstallation {
    let product_root = root.join("World of Warcraft");
    let flavor_root = product_root.join("_retail_");
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir).expect("addon root");
    fs::create_dir_all(&wtf_dir).expect("wtf root");
    fs::create_dir_all(&fonts_dir).expect("fonts root");

    if with_content {
        fs::create_dir_all(addon_dir.join("WeakAuras")).expect("addon dir");
        fs::write(
            addon_dir.join("WeakAuras").join("WeakAuras.toc"),
            "## Interface: 110000",
        )
        .expect("toc");
        fs::write(
            addon_dir.join("WeakAuras").join("WeakAuras.lua"),
            r#"
WeakAurasSaved = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["player"] = "Examplemage",
}
"#,
        )
        .expect("addon lua");

        fs::write(wtf_dir.join("Config.wtf"), "SET locale enUS").expect("config");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("SavedVariables"),
        )
        .expect("saved variables");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("SavedVariables")
                .join("Details.lua"),
            r#"
DetailsDB = {
  ["profileKeys"] = {
    ["Examplemage - Illidan"] = "Default",
  },
  ["profiles"] = {
    ["Default.Illidan.Examplemage"] = {},
  },
}
"#,
        )
        .expect("saved variable");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage"),
        )
        .expect("character");
        fs::create_dir_all(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("SavedVariables"),
        )
        .expect("character saved variables");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("AddOns.txt"),
            "WeakAuras: enabled",
        )
        .expect("addons state");
        fs::write(
            wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("Illidan")
                .join("Examplemage")
                .join("SavedVariables")
                .join("Pawn.lua"),
            r#"
PawnOptions = {
  ["LastPlayerFullName"] = "Examplemage",
  ["LastRealm"] = "Illidan",
}
"#,
        )
        .expect("character lua");

        fs::write(fonts_dir.join("FRIZQT__.ttf"), "font").expect("font");
        fs::create_dir_all(interface_dir.join("SharedXML")).expect("asset dir");
        fs::write(
            interface_dir.join("SharedXML").join("texture.blp"),
            "texture",
        )
        .expect("asset");
    }

    DetectedFlavorInstallation {
        platform,
        product_root,
        flavor_root,
        flavor: WowFlavor::Retail,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn seed_external_package_policy_target(installation: &DetectedFlavorInstallation) {
    fs::create_dir_all(installation.addon_dir.join("WeakAuras")).expect("addon dir");
    fs::write(
        installation.addon_dir.join("WeakAuras").join("Stale.lua"),
        "print('stale')",
    )
    .expect("stale addon");

    fs::write(installation.wtf_dir.join("Config.wtf"), "SET locale zhCN").expect("config");
    fs::create_dir_all(
        installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("SavedVariables"),
    )
    .expect("character dir");
    fs::write(
        installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("StaleCharacter.txt"),
        "stale-character",
    )
    .expect("stale character root");
    fs::write(
        installation
            .wtf_dir
            .join("Account")
            .join("ACCOUNT")
            .join("Illidan")
            .join("Examplemage")
            .join("SavedVariables")
            .join("Old.lua"),
        "OldSaved = true",
    )
    .expect("stale character saved variables");

    fs::write(installation.fonts_dir.join("FRIZQT__.ttf"), "mac-font").expect("font");
    fs::create_dir_all(installation.interface_dir.join("SharedXML")).expect("shared xml");
    fs::write(
        installation.interface_dir.join("SharedXML").join("old.blp"),
        "old-texture",
    )
    .expect("old texture");
}

fn bundle_testdata_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("core")
        .join("bundle")
        .join("testdata")
        .join(name)
}

fn external_package_fixture_root() -> std::path::PathBuf {
    bundle_testdata_path("external_package_author_ui_wrapped")
}

fn external_package_dirty_fixture_root() -> std::path::PathBuf {
    bundle_testdata_path("external_package_dirty_mixed_case")
}

fn external_package_conflict_fixture_root() -> std::path::PathBuf {
    bundle_testdata_path("external_package_conflicting_duplicates")
}

fn create_external_package_fixture_archive(archive_path: &std::path::Path) {
    create_archive_from_directory(&external_package_fixture_root(), archive_path);
}

fn create_archive_from_directory(source_root: &std::path::Path, archive_path: &std::path::Path) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    add_directory_entries_to_zip(&mut zip, source_root, source_root);
    zip.finish().expect("finish archive");
}

fn create_archive_with_raw_entries(archive_path: &std::path::Path, entries: &[(&str, &str)]) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for (name, content) in entries {
        zip.start_file(
            *name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start raw archive file");
        zip.write_all(content.as_bytes())
            .expect("write raw archive file");
    }
    zip.finish().expect("finish raw archive");
}

fn create_archive_with_raw_directories(archive_path: &std::path::Path, entries: &[&str]) {
    let file = fs::File::create(archive_path).expect("archive file");
    let mut zip = ZipWriter::new(file);
    for name in entries {
        zip.add_directory(
            *name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .expect("add raw archive directory");
    }
    zip.finish().expect("finish raw directory archive");
}

fn add_directory_entries_to_zip(
    zip: &mut ZipWriter<fs::File>,
    source_root: &std::path::Path,
    current: &std::path::Path,
) {
    let mut entries = fs::read_dir(current)
        .expect("read dir")
        .map(|entry| entry.expect("dir entry").path())
        .collect::<Vec<_>>();
    entries.sort();

    for entry_path in entries {
        if entry_path.is_dir() {
            add_directory_entries_to_zip(zip, source_root, &entry_path);
            continue;
        }

        let archive_name = entry_path
            .strip_prefix(source_root)
            .expect("relative fixture path")
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(
            archive_name,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )
        .expect("start fixture file");
        zip.write_all(&fs::read(&entry_path).expect("fixture bytes"))
            .expect("write fixture file");
    }
}

fn create_addon_archive(path: &std::path::Path, entries: &[(&str, &str)]) {
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

fn sample_manifest() -> BundleManifest {
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
            wtf_common: true,
            wtf_characters: vec![CharacterResource {
                source_account: Some("ACCOUNT".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Examplemage".to_string(),
                target_hint: None,
            }],
            fonts: true,
            interface_assets: vec!["SharedXML".to_string()],
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

fn sample_manifest_with_rewrite() -> BundleManifest {
    let mut manifest = sample_manifest();
    manifest.mapping.rewrite_profile_keys = true;
    manifest.mapping.rewrite_identity_strings = true;
    manifest
}

fn sample_external_package_request_with_apply_defaults(
    source_path: std::path::PathBuf,
    apply_defaults: Option<ApplyDefaults>,
) -> CreateExternalPackageBundleRequest {
    CreateExternalPackageBundleRequest {
        source_path,
        source_flavor: WowFlavor::Retail,
        source_platform: Some(HostPlatform::Windows),
        supported_targets: vec![WowFlavor::Retail],
        output_path: None,
        package_id: Some("author-ui-import".to_string()),
        package_name: Some("Author UI Import".to_string()),
        created_by: Some("hearthsync-test".to_string()),
        description: Some("fixture external package".to_string()),
        apply_defaults,
    }
}
