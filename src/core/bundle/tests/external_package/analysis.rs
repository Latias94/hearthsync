use super::*;

#[test]
fn analyze_external_package_zip_normalizes_wrapped_ui_layout() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("author-ui-pack.zip");

    create_external_package_fixture_archive(&package_path);

    let analysis =
        analyze_external_package(AnalyzeExternalPackageRequest::new(package_path.clone()))
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
    assert!(analysis.summary.sensitive_wtf_files.iter().any(|file| {
        file.kind == ExternalPackageSensitiveWtfFileKind::SavedVariables
            && file.severity == ExternalPackagePublicSharingSeverity::ReviewRequired
            && file.count == 2
    }));
    assert!(analysis.summary.sensitive_wtf_files.iter().any(|file| {
        file.kind == ExternalPackageSensitiveWtfFileKind::Bindings
            && file.severity == ExternalPackagePublicSharingSeverity::Advisory
            && file.count == 1
    }));

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

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect("analyze external package with archive noise");

    assert_eq!(analysis.summary.total_files, 1);
    assert_eq!(analysis.summary.normalized_files, 1);
    assert_eq!(analysis.summary.ignored_files, 0);
    assert_eq!(analysis.summary.warning_count, 0);
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert_eq!(
        analysis.summary.public_sharing.status,
        ExternalPackagePublicSharingStatus::Ready
    );
    assert!(analysis.summary.public_sharing.public_ready);
    assert!(analysis.summary.public_sharing.reasons.is_empty());
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

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_root))
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

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
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

    let analysis =
        analyze_external_package(AnalyzeExternalPackageRequest::new(package_root.clone()))
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

    let analysis =
        analyze_external_package(AnalyzeExternalPackageRequest::new(package_root.clone()))
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
    assert_eq!(
        analysis.summary.wtf_scopes,
        vec![
            ExternalPackageWtfScopeSummary {
                scope: WtfScope::RootSavedVariables,
                risk: WtfScopeRisk::High,
                count: 1,
            },
            ExternalPackageWtfScopeSummary {
                scope: WtfScope::CharacterSavedVariables,
                risk: WtfScopeRisk::Medium,
                count: 1,
            },
            ExternalPackageWtfScopeSummary {
                scope: WtfScope::CacheLike,
                risk: WtfScopeRisk::Low,
                count: 1,
            },
        ]
    );
    assert_eq!(
        analysis.summary.source_identities.source_accounts,
        vec!["ACC1"]
    );
    assert_eq!(
        analysis.summary.source_identities.source_characters,
        vec![ExternalPackageSourceCharacterSummary {
            source_account: Some("ACC1".to_string()),
            source_server: "Illidan".to_string(),
            source_character: "Targetone".to_string(),
        }]
    );
    assert_eq!(
        analysis
            .summary
            .source_identities
            .entries_with_source_account,
        2
    );
    assert_eq!(
        analysis
            .summary
            .source_identities
            .entries_with_source_character,
        1
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

    let analysis =
        analyze_external_package(AnalyzeExternalPackageRequest::new(package_path.clone()))
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

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_root))
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
fn analyze_external_package_directory_detects_direct_addons_and_root_savedvariables() {
    let temp = tempdir().expect("temp dir");
    let package_root = temp.path().join("AuthorPack");

    fs::create_dir_all(package_root.join("WeakAuras")).expect("addon dir");
    fs::write(
        package_root.join("WeakAuras").join("WeakAuras.toc"),
        "## Interface: 110000",
    )
    .expect("addon toc");
    fs::create_dir_all(package_root.join("WTF").join("SavedVariables")).expect("root saved dir");
    fs::write(
        package_root
            .join("WTF")
            .join("SavedVariables")
            .join("Blizzard_Console.lua"),
        "Console = true",
    )
    .expect("root saved variables file");
    fs::write(
        package_root
            .join("WTF")
            .join("SavedVariables")
            .join("Blizzard_Console.lua.bak"),
        "ConsoleBackup = true",
    )
    .expect("root saved variables backup");
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

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_root))
        .expect("analyze external package directory");

    assert_eq!(analysis.source_kind, ExternalPackageSourceKind::Directory);
    assert_eq!(analysis.resources.addons, vec!["WeakAuras".to_string()]);
    assert!(analysis.resources.fonts);
    assert!(analysis.resources.wtf_common);
    assert_eq!(analysis.summary.total_files, 5);
    assert_eq!(analysis.summary.normalized_files, 5);
    assert_eq!(analysis.summary.ignored_files, 0);
    assert_eq!(analysis.summary.warning_count, 0);
    assert_eq!(analysis.summary.wtf_warning_count, 0);
    assert!(analysis.summary.warning_groups.is_empty());
    assert!(analysis.warnings.is_empty());
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/root/SavedVariables/Blizzard_Console.lua"
            && entry.wtf_scope == Some(WtfScope::RootSavedVariables)
    }));
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/root/SavedVariables/Blizzard_Console.lua.bak"
            && entry.wtf_scope == Some(WtfScope::RootSavedVariables)
    }));
    assert!(analysis.entries.iter().any(|entry| {
        entry.normalized_path == "wtf/common/root/SavedVariables/Broken.lua"
            && entry.wtf_scope == Some(WtfScope::RootSavedVariables)
    }));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_addon_with_mixed_separators() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("unknown_plug-postal.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            (
                "Postal/Postal.toc",
                "## Interface: 110000\n## Title: Postal\n",
            ),
            ("Postal/Libs\\AceAddon-3.0\\AceAddon-3.0.lua", "ace"),
        ],
    );

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect("analyze NewBeeBox addon package");

    assert_eq!(analysis.layout, ExternalPackageLayout::NewBeeBoxAddon);
    assert_eq!(analysis.resources.addons, vec!["Postal".to_string()]);
    assert_eq!(analysis.summary.total_files, 2);
    assert_eq!(analysis.summary.normalized_files, 2);
    assert!(analysis.entries.iter().any(|entry| {
        entry.source_path == "Postal/Libs\\AceAddon-3.0\\AceAddon-3.0.lua"
            && entry.normalized_path == "addons/Postal/Libs/AceAddon-3.0/AceAddon-3.0.lua"
    }));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_module_cache_addon_with_mixed_separators() {
    let temp = tempdir().expect("temp dir");
    let package_dir = temp.path().join("NewBeeBoxCache").join("modules");
    fs::create_dir_all(&package_dir).expect("NewBeeBox modules dir");
    let package_path = package_dir.join("11225-7685_164-MeetingStone.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            (
                "MeetingStone/MeetingStone.toc",
                "## Interface: 110000\n## Title: MeetingStone\n",
            ),
            ("MeetingStone\\addon_version.txt", "12.1.4"),
        ],
    );

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect("analyze NewBeeBox module cache package");

    assert_eq!(analysis.layout, ExternalPackageLayout::NewBeeBoxAddon);
    assert_eq!(analysis.resources.addons, vec!["MeetingStone".to_string()]);
    assert_eq!(analysis.summary.total_files, 2);
    assert_eq!(analysis.summary.normalized_files, 2);
    assert!(analysis.entries.iter().any(|entry| {
        entry.source_path == "MeetingStone\\addon_version.txt"
            && entry.normalized_path == "addons/MeetingStone/addon_version.txt"
    }));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_flat_font_package() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("font-example.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("FRIZQT__.TTF", "font-a"), ("ARIALN.TTF", "font-b")],
    );

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect("analyze NewBeeBox font package");

    assert_eq!(analysis.layout, ExternalPackageLayout::NewBeeBoxFont);
    assert!(analysis.resources.fonts);
    assert_eq!(analysis.summary.fonts, 2);
    let normalized_paths = normalized_paths(&analysis);
    assert!(normalized_paths.contains(&"fonts/FRIZQT__.TTF"));
    assert!(normalized_paths.contains(&"fonts/ARIALN.TTF"));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_module_cache_font_package() {
    let temp = tempdir().expect("temp dir");
    let package_dir = temp.path().join("NewBeeBoxCache").join("modules");
    fs::create_dir_all(&package_dir).expect("NewBeeBox modules dir");
    let package_path = package_dir.join("font-0fa800a722f834cd6e55938c278e5c27.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("Fonts/FRIZQT__.TTF", "font-a"), ("ARIALN.TTF", "font-b")],
    );

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect("analyze NewBeeBox module font package");

    assert_eq!(analysis.layout, ExternalPackageLayout::NewBeeBoxFont);
    assert!(analysis.resources.fonts);
    assert_eq!(analysis.summary.fonts, 2);
    assert_eq!(analysis.summary.normalized_files, 2);
    let normalized_paths = normalized_paths(&analysis);
    assert!(normalized_paths.contains(&"fonts/FRIZQT__.TTF"));
    assert!(normalized_paths.contains(&"fonts/ARIALN.TTF"));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_flat_material_package() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("material-example.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("Icons/icon.blp", "icon"),
            ("SimpleChatEmojis/Menu\\arrow.tga", "arrow"),
        ],
    );

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect("analyze NewBeeBox material package");

    assert_eq!(analysis.layout, ExternalPackageLayout::NewBeeBoxMaterial);
    assert_eq!(
        analysis.resources.interface_assets,
        vec!["Icons".to_string(), "SimpleChatEmojis".to_string()]
    );
    let normalized_paths = normalized_paths(&analysis);
    assert!(normalized_paths.contains(&"interface/Icons/icon.blp"));
    assert!(normalized_paths.contains(&"interface/SimpleChatEmojis/Menu/arrow.tga"));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_module_cache_material_package() {
    let temp = tempdir().expect("temp dir");
    let package_dir = temp.path().join("NewBeeBoxCache").join("modules");
    fs::create_dir_all(&package_dir).expect("NewBeeBox modules dir");
    let package_path = package_dir.join("material-3613b41981fdab61b470798e1b71e0a1.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("Interface/Icons/icon.blp", "icon"),
            ("SimpleChatEmojis/Menu\\arrow.tga", "arrow"),
        ],
    );

    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect("analyze NewBeeBox module material package");

    assert_eq!(analysis.layout, ExternalPackageLayout::NewBeeBoxMaterial);
    assert_eq!(analysis.summary.interface_assets, 2);
    assert_eq!(analysis.summary.normalized_files, 2);
    assert_eq!(
        analysis.resources.interface_assets,
        vec!["Icons".to_string(), "SimpleChatEmojis".to_string()]
    );
    let normalized_paths = normalized_paths(&analysis);
    assert!(normalized_paths.contains(&"interface/Icons/icon.blp"));
    assert!(normalized_paths.contains(&"interface/SimpleChatEmojis/Menu/arrow.tga"));
}

#[test]
fn analyze_external_package_newbeebox_account_wtf_requires_source_account() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("wtfserve-example.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[("SavedVariables/Details.lua", "DetailsDB = {}")],
    );

    let error = analyze_external_package(AnalyzeExternalPackageRequest::new(package_path))
        .expect_err("account WTF package without source account should fail");

    assert!(error.to_string().contains("source_account"));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_flat_account_wtf_package() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("wtfserve-example.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("bindings-cache.wtf", "bindings"),
            ("SavedVariables/Details.lua", "DetailsDB = {}"),
        ],
    );

    let mut request = AnalyzeExternalPackageRequest::new(package_path);
    request.source_account = Some("ACCOUNT".to_string());
    let analysis = analyze_external_package(request).expect("analyze NewBeeBox account WTF");

    assert_eq!(analysis.layout, ExternalPackageLayout::NewBeeBoxWtfAccount);
    assert!(analysis.resources.wtf_common);
    let normalized_paths = normalized_paths(&analysis);
    assert!(normalized_paths.contains(&"wtf/common/accounts/ACCOUNT/bindings-cache.wtf"));
    assert!(normalized_paths.contains(&"wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua"));
}

#[test]
fn analyze_external_package_auto_detects_newbeebox_flat_character_wtf_package() {
    let temp = tempdir().expect("temp dir");
    let package_path = temp.path().join("wtfrole-example.zip");
    create_archive_with_raw_entries(
        &package_path,
        &[
            ("Sourcechar/AddOns.txt", "addons"),
            ("Sourcechar/SavedVariables\\Pawn.lua", "PawnOptions = {}"),
        ],
    );

    let mut request = AnalyzeExternalPackageRequest::new(package_path);
    request.source_account = Some("ACCOUNT".to_string());
    request.source_server = Some("Illidan".to_string());
    let analysis = analyze_external_package(request).expect("analyze NewBeeBox character WTF");

    assert_eq!(
        analysis.layout,
        ExternalPackageLayout::NewBeeBoxWtfCharacter
    );
    assert_eq!(analysis.resources.wtf_characters.len(), 1);
    assert_eq!(
        analysis.resources.wtf_characters[0].source_account,
        Some("ACCOUNT".to_string())
    );
    assert_eq!(
        analysis.resources.wtf_characters[0].source_server,
        "Illidan"
    );
    assert_eq!(
        analysis.resources.wtf_characters[0].source_character,
        "Sourcechar"
    );
    let normalized_paths = normalized_paths(&analysis);
    assert!(normalized_paths.contains(&"wtf/characters/ACCOUNT/Illidan/Sourcechar/AddOns.txt"));
    assert!(
        normalized_paths
            .contains(&"wtf/characters/ACCOUNT/Illidan/Sourcechar/SavedVariables/Pawn.lua")
    );
}

fn normalized_paths(analysis: &ExternalPackageAnalysis) -> Vec<&str> {
    analysis
        .entries
        .iter()
        .map(|entry| entry.normalized_path.as_str())
        .collect()
}
