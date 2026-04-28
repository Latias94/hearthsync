use std::fs;

use tempfile::tempdir;

use super::support::*;
use crate::core::bundle::*;
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::manifest::{ApplyDefaults, CharacterMappingMode, ResourceApplyPolicy};
use crate::core::task::{NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

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
    assert_eq!(analysis.source_kind, ExternalPackageSourceKind::ZipArchive);
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
fn analyze_external_package_zip_ignores_macos_metadata_and_desktop_noise() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("author-ui-pack-with-noise.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            (
                "AuthorUI/Interface/AddOns/WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Title: WeakAuras\n",
            ),
            (
                "__MACOSX/AuthorUI/Interface/AddOns/WeakAuras/._WeakAuras.toc",
                "resource fork",
            ),
            (
                "AuthorUI/Interface/AddOns/WeakAuras/._WeakAuras.lua",
                "resource fork",
            ),
            ("AuthorUI/Fonts/._FRIZQT__.ttf", "resource fork"),
            ("AuthorUI/Interface/AddOns/WeakAuras/.DS_Store", "noise"),
            ("AuthorUI/Interface/AddOns/WeakAuras/Thumbs.db", "noise"),
            ("AuthorUI/desktop.ini", "noise"),
        ],
    );

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect("analyze external package with archive noise");

    assert_eq!(analysis.summary.total_files, 1);
    assert_eq!(analysis.summary.normalized_files, 1);
    assert_eq!(analysis.summary.ignored_files, 0);
    assert_eq!(analysis.summary.warning_count, 0);
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert_eq!(analysis.entries.len(), 1);
    assert_eq!(
        analysis.entries[0].normalized_path,
        "addons/WeakAuras/WeakAuras.toc"
    );
    assert!(
        analysis
            .entries
            .iter()
            .all(|entry| !entry.source_path.contains("__MACOSX"))
    );
    assert!(
        analysis
            .entries
            .iter()
            .all(|entry| !entry.source_path.contains("/._"))
    );
}

#[test]
fn analyze_external_package_directory_ignores_appledouble_sidecars() {
    let temp = tempdir().expect("temp dir");
    let package_root = temp.path().join("AuthorUI");
    let addon_root = package_root
        .join("Interface")
        .join("AddOns")
        .join("WeakAuras");

    fs::create_dir_all(&addon_root).expect("addon root");
    fs::write(
        addon_root.join("WeakAuras.toc"),
        "## Interface: 110000\n## Title: WeakAuras\n",
    )
    .expect("toc");
    fs::write(addon_root.join("._WeakAuras.toc"), "resource fork").expect("sidecar");

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root,
    })
    .expect("analyze external package directory with sidecar");

    assert_eq!(analysis.summary.total_files, 1);
    assert_eq!(analysis.summary.normalized_files, 1);
    assert_eq!(analysis.summary.ignored_files, 0);
    assert_eq!(analysis.summary.warning_count, 0);
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert_eq!(analysis.entries.len(), 1);
    assert_eq!(
        analysis.entries[0].normalized_path,
        "addons/WeakAuras/WeakAuras.toc"
    );
}

#[test]
fn analyze_external_package_zip_handles_large_wrapped_author_package() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("large-author-ui-pack.zip");
    let mut entries = Vec::new();
    let mut expected_addons = Vec::new();

    for addon_index in 0..12 {
        let addon_name = format!("Addon{addon_index:02}");
        expected_addons.push(addon_name.clone());
        entries.push((
            format!("AuthorUI/Interface/AddOns/{addon_name}/{addon_name}.toc"),
            format!("## Interface: 110000\n## Title: {addon_name}\n"),
        ));

        for module_index in 0..8 {
            entries.push((
                format!("AuthorUI/Interface/AddOns/{addon_name}/Modules/Module{module_index}.lua"),
                format!("print('{addon_name}:{module_index}')"),
            ));
        }

        entries.push((
            format!("AuthorUI/Interface/AddOns/{addon_name}/.DS_Store"),
            "noise".to_string(),
        ));
        entries.push((
            format!("__MACOSX/AuthorUI/Interface/AddOns/{addon_name}/._{addon_name}.toc"),
            "resource fork".to_string(),
        ));
    }

    entries.extend([
        (
            "AuthorUI/WTF/Config.wtf".to_string(),
            "SET locale enUS".to_string(),
        ),
        (
            "AuthorUI/WTF/Account/ACCOUNT/SavedVariables/Details.lua".to_string(),
            "DetailsDB = {}".to_string(),
        ),
        (
            "AuthorUI/WTF/Account/ACCOUNT/Illidan/Examplemage/AddOns.txt".to_string(),
            "addons".to_string(),
        ),
        (
            "AuthorUI/WTF/Account/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua".to_string(),
            "PawnOptions = {}".to_string(),
        ),
        (
            "AuthorUI/Fonts/FRIZQT__.ttf".to_string(),
            "font".to_string(),
        ),
        (
            "AuthorUI/Interface/SharedXML/texture.blp".to_string(),
            "texture".to_string(),
        ),
    ]);
    create_archive_with_owned_raw_entries(&package_path, &entries);

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect("analyze large author package");

    assert_eq!(analysis.source_kind, ExternalPackageSourceKind::ZipArchive);
    assert_eq!(analysis.summary.total_files, 114);
    assert_eq!(analysis.summary.normalized_files, 114);
    assert_eq!(analysis.summary.warning_count, 0);
    assert_eq!(analysis.summary.addons, 108);
    assert_eq!(analysis.summary.wtf_common, 2);
    assert_eq!(analysis.summary.wtf_characters, 2);
    assert_eq!(analysis.summary.fonts, 1);
    assert_eq!(analysis.summary.interface_assets, 1);
    assert_eq!(analysis.resources.addons, expected_addons);
    assert!(analysis.resources.wtf_common);
    assert_eq!(analysis.resources.wtf_characters.len(), 1);
    assert!(analysis.resources.fonts);
    assert_eq!(
        analysis.resources.interface_assets,
        vec!["SharedXML".to_string()]
    );
    assert!(analysis.warnings.is_empty());
    assert!(
        analysis
            .entries
            .iter()
            .all(|entry| !entry.source_path.contains("__MACOSX"))
    );
    assert!(
        analysis
            .entries
            .iter()
            .all(|entry| !entry.source_path.contains(".DS_Store"))
    );
}

#[test]
fn analyze_external_package_directory_fixture_normalizes_wrapped_ui_layout() {
    let package_root = external_package_fixture_root();

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_root.clone(),
    })
    .expect("analyze external package directory fixture");

    assert_eq!(analysis.source_path, package_root);
    assert_eq!(analysis.source_kind, ExternalPackageSourceKind::Directory);
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
    assert_eq!(analysis.source_kind, ExternalPackageSourceKind::Directory);
    assert_eq!(analysis.package_id, "external_package_dirty_mixed_case");
    assert_eq!(analysis.summary.total_files, 8);
    assert_eq!(analysis.summary.normalized_files, 7);
    assert_eq!(analysis.summary.ignored_files, 1);
    assert_eq!(analysis.summary.warning_count, 1);
    assert_eq!(analysis.summary.addon_warning_count, 1);
    assert_eq!(analysis.summary.wtf_warning_count, 0);
    assert_eq!(
        analysis.summary.warning_groups,
        vec![ExternalPackageWarningGroup {
            category: ExternalPackageWarningCategory::Addon,
            code: ExternalPackageWarningCode::AddonRootNotDetected,
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
        warning.code == ExternalPackageWarningCode::AddonRootNotDetected
            && warning.message.contains("no addon root was detected")
            && warning.source_path.contains("BrokenAddon/README.txt")
    }));

    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/root/SavedVariables/Broken.lua"
            && entry.wtf_scope == Some(WtfScope::RootSavedVariables)
    }));
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/accounts/ACC1/config-cache.wtf"
            && entry.wtf_scope == Some(WtfScope::CacheLike)
    }));
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path
            == "wtf/characters/ACC1/Illidan/Targetone/SavedVariables/MeetingStone.lua"
            && entry.wtf_scope == Some(WtfScope::CharacterSavedVariables)
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
    assert_eq!(analysis.source_kind, ExternalPackageSourceKind::ZipArchive);
    assert_eq!(analysis.package_id, "dirty-author-pack");
    assert_eq!(analysis.summary.total_files, 8);
    assert_eq!(analysis.summary.normalized_files, 7);
    assert_eq!(analysis.summary.ignored_files, 1);
    assert_eq!(analysis.summary.warning_count, 1);
    assert_eq!(
        analysis.summary.warning_groups,
        vec![ExternalPackageWarningGroup {
            category: ExternalPackageWarningCategory::Addon,
            code: ExternalPackageWarningCode::AddonRootNotDetected,
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
            && entry.group == ApplyGroup::Addons
    }));
}

#[test]
fn apply_external_package_keeps_case_mixed_addon_subtree_on_macos_target() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_root = source.path().join("AuthorUI");
    let exact_case_root = package_root
        .join("Interface")
        .join("AddOns")
        .join("WeakAuras");
    let mixed_case_root = package_root
        .join("Interface")
        .join("AddOns")
        .join("weakauras");

    fs::create_dir_all(&exact_case_root).expect("exact addon root");
    fs::create_dir_all(mixed_case_root.join("Modules")).expect("mixed-case addon subtree");
    fs::write(
        exact_case_root.join("WeakAuras.toc"),
        "## Interface: 110000\n## Title: WeakAuras\n",
    )
    .expect("toc");
    fs::write(mixed_case_root.join("Core.lua"), "print('wa core')").expect("core");
    fs::write(
        mixed_case_root.join("Modules").join("Module.lua"),
        "print('wa module')",
    )
    .expect("module");

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    let result = apply_external_package(ApplyExternalPackageRequest {
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
            apply_defaults: Some(ApplyDefaults {
                create_backup: false,
                addons: ResourceApplyPolicy::Mirror,
                wtf_common: ResourceApplyPolicy::Share,
                wtf_characters: ResourceApplyPolicy::ReplaceSelected,
                fonts: ResourceApplyPolicy::Mirror,
                interface_assets: ResourceApplyPolicy::Mirror,
            }),
        },
        installation: target_installation.clone(),
        dry_run: false,
        backup_output_path: None,
        apply_mappings: BundleApplyMappings::default(),
    })
    .expect("apply external package with case-mixed addon subtree");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert!(!result.dry_run);
    assert_eq!(result.written_files, 3);
    assert_eq!(
        fs::read_to_string(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("Core.lua")
        )
        .expect("core"),
        "print('wa core')"
    );
    assert_eq!(
        fs::read_to_string(
            target_installation
                .addon_dir
                .join("WeakAuras")
                .join("Modules")
                .join("Module.lua")
        )
        .expect("module"),
        "print('wa module')"
    );
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
fn analyze_external_package_rejects_zip_with_windows_reserved_segments() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unsafe-reserved-segment.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("AuthorUI/Interface/AddOns/Weak:Auras/WeakAuras.toc", "toc")],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("reserved Windows path segment should be rejected");

    assert!(error.to_string().contains("unsafe archive path"));
}

#[test]
fn analyze_external_package_rejects_zip_symlink_entries() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("symlink-author-pack.zip");
    create_archive_with_symlink_entry(
        &package_path,
        "AuthorUI/Interface/AddOns/WeakAuras/WeakAuras.lua",
        "../../outside.lua",
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest {
        source_path: package_path,
    })
    .expect_err("symlink zip entry should be rejected");

    let message = error.to_string();
    assert!(message.contains("unsupported symlink metadata"));
    assert!(message.contains("AuthorUI/Interface/AddOns/WeakAuras/WeakAuras.lua"));
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

    assert_eq!(analysis.source_kind, ExternalPackageSourceKind::Directory);
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
            && entry.wtf_scope == Some(WtfScope::RootSavedVariables)
    }));
}

#[test]
fn analyze_external_package_directory_rejects_non_portable_relative_segments() {
    let error = crate::core::archive_path::safe_relative_segments(
        std::path::Path::new("AuthorUI/Interface/AddOns/Weak:Auras/WeakAuras.toc"),
        "directory entry path",
    )
    .expect_err("non-portable directory segment should fail");

    assert!(error.to_string().contains("unsafe directory entry path"));
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
            .any(|item| item.group == ApplyGroup::Addons)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == ApplyGroup::WtfCommon)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == ApplyGroup::WtfCharacters)
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
            .any(|item| item.group == ApplyGroup::Addons)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == ApplyGroup::WtfCommon)
    );
    assert!(
        plan.operations
            .iter()
            .any(|item| item.group == ApplyGroup::WtfCharacters)
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
fn plan_external_package_apply_uses_author_package_default_profile_when_apply_defaults_missing() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: sample_external_package_request_with_apply_defaults(package_path, None),
        installation: target_installation.clone(),
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package apply with default profile");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert!(plan.manifest.apply.create_backup);
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
        ResourceApplyPolicy::Mirror
    );
    assert_eq!(
        plan.group_policies.interface_assets.policy,
        ResourceApplyPolicy::Mirror
    );
    assert_eq!(plan.selected_target_accounts, vec!["ACCOUNT".to_string()]);
    assert_eq!(plan.summary.paths_to_remove, 4);
    assert_eq!(plan.summary.files_to_add, 8);
    assert_eq!(plan.summary.files_to_replace, 0);
    assert_eq!(plan.summary.files_to_skip, 0);
    assert_eq!(plan.summary.files_to_preserve, 1);

    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.fonts_dir
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.interface_dir.join("SharedXML")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
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
            && operation.action == ApplyAction::Preserve
    }));
}

#[test]
fn external_package_apply_plan_does_not_expose_execution_only_fields() {
    let package_root = external_package_fixture_root();
    let target = tempdir().expect("target temp dir");
    let installation = create_fixture_installation(target.path(), false);

    let plan = plan_external_package_apply(PlanExternalPackageApplyRequest {
        external_package: sample_external_package_request_with_apply_defaults(package_root, None),
        installation,
        apply_mappings: BundleApplyMappings {
            target_account: Some("ACCOUNT".to_string()),
            target_server: Some("Illidan".to_string()),
            target_character: Some("Examplemage".to_string()),
            ..BundleApplyMappings::default()
        },
    })
    .expect("plan external package");

    let serialized_plan = serde_json::to_value(&plan).expect("serialize external package plan");
    let operations = serialized_plan
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
    assert!(serialized_plan.get("prepared_apply").is_none());
    assert!(serialized_plan.get("apply_source").is_none());
    assert!(serialized_plan.get("entry_source_map").is_none());
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
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.addon_dir.join("WeakAuras")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
            && operation.destination == target_installation.interface_dir.join("SharedXML")
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.action == ApplyAction::Remove
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
            && operation.action == ApplyAction::Preserve
    }));
    assert!(plan.operations.iter().any(|operation| {
        operation.archive_name == "fonts/FRIZQT__.ttf" && operation.action == ApplyAction::Preserve
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
fn apply_external_package_applies_complex_windows_author_zip_to_macos_target() {
    let source = tempdir().expect("source temp dir");
    let target = tempdir().expect("target temp dir");
    let package_path = source.path().join("complex-author-ui-pack.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            (
                "AuthorUI/Interface/AddOns/WeakAuras/WeakAuras.toc",
                "## Interface: 110000\n## Title: WeakAuras\n",
            ),
            (
                "AuthorUI/INTERFACE/ADDONS/weakauras/Core.lua",
                "print('core')",
            ),
            (
                "AuthorUI/INTERFACE/ADDONS/weakauras/Modules/Module.lua",
                "print('module')",
            ),
            (
                "__MACOSX/AuthorUI/Interface/AddOns/WeakAuras/._WeakAuras.toc",
                "resource fork",
            ),
            (
                "AuthorUI/INTERFACE/ADDONS/weakauras/._Core.lua",
                "resource fork",
            ),
            ("AuthorUI/Interface/AddOns/WeakAuras/.DS_Store", "noise"),
            ("AuthorUI/WTF/Config.wtf", "SET locale enUS"),
            (
                "AuthorUI/WTF/Account/ACCOUNT/SavedVariables/Details.lua",
                "DetailsDB = {}",
            ),
            (
                "AuthorUI/WTF/Account/ACCOUNT/bindings-cache.wtf",
                "target bindings",
            ),
            (
                "AuthorUI/WTF/Account/ACCOUNT/Illidan/Examplemage/AddOns.txt",
                "WeakAuras: enabled",
            ),
            (
                "AuthorUI/WTF/Account/ACCOUNT/Illidan/Examplemage/SavedVariables/Pawn.lua",
                "PawnOptions = {}",
            ),
            ("AuthorUI/Fonts/FRIZQT__.ttf", "complex-font"),
            ("AuthorUI/Fonts/._FRIZQT__.ttf", "resource fork"),
            (
                "AuthorUI/INTERFACE/SharedXML/texture.blp",
                "complex-texture",
            ),
            ("AuthorUI/INTERFACE/SharedXML/Thumbs.db", "noise"),
        ],
    );

    let target_installation =
        create_fixture_installation_on_platform(target.path(), false, HostPlatform::MacOs);
    seed_external_package_policy_target(&target_installation);

    let result = apply_external_package(ApplyExternalPackageRequest {
        external_package: sample_external_package_request_with_apply_defaults(package_path, None),
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
    .expect("apply complex external package");

    assert_eq!(target_installation.platform, HostPlatform::MacOs);
    assert_eq!(result.manifest.source.platform, Some(HostPlatform::Windows));
    assert!(result.manifest.apply.create_backup);
    assert!(result.backup_path.is_some());
    assert_eq!(result.analysis.summary.total_files, 10);
    assert_eq!(result.analysis.summary.normalized_files, 10);
    assert_eq!(result.analysis.summary.warning_count, 0);
    assert_eq!(
        result.analysis.resources.addons,
        vec!["WeakAuras".to_string()]
    );
    assert_eq!(
        result.analysis.resources.interface_assets,
        vec!["SharedXML".to_string()]
    );
    assert_eq!(result.written_files, 9);
    assert_eq!(result.plan_summary.paths_to_remove, 4);
    assert_eq!(result.plan_summary.files_to_preserve, 1);

    let addon_root = target_installation.addon_dir.join("WeakAuras");
    assert!(addon_root.join("WeakAuras.toc").exists());
    assert_eq!(
        fs::read_to_string(addon_root.join("Core.lua")).expect("core"),
        "print('core')"
    );
    assert_eq!(
        fs::read_to_string(addon_root.join("Modules").join("Module.lua")).expect("module"),
        "print('module')"
    );
    assert!(!addon_root.join("Stale.lua").exists());
    assert!(!addon_root.join("._Core.lua").exists());
    assert!(!addon_root.join(".DS_Store").exists());

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
        "complex-font"
    );
    assert!(
        !target_installation
            .fonts_dir
            .join("._FRIZQT__.ttf")
            .exists()
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
        .expect("texture"),
        "complex-texture"
    );
    assert!(
        !target_installation
            .interface_dir
            .join("SharedXML")
            .join("Thumbs.db")
            .exists()
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
        ExternalPackageWarningCode::AddonRootNotDetected,
        ExternalPackageWarningCode::UnsupportedWtfLayout,
        ExternalPackageWarningCode::WtfAccountPathWithoutFile,
        ExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile,
        ExternalPackageWarningCode::UnsupportedWtfNestedAccountLayout,
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
