use super::*;

#[test]
fn render_addon_search_catalog_lists_results() {
    let rendered = render_addon_search_catalog(&AddonSearchCatalogResult {
        query: "weakauras".to_string(),
        provider_id: None,
        result_count: 1,
        failure_count: 0,
        results: vec![AddonSearchResult {
            provider: "curseforge".to_string(),
            name: "WeakAuras".to_string(),
            summary: Some("Aura tracking".to_string()),
            source: sample_source(),
            source_label: "curseforge:123".to_string(),
            install_hint: "curseforge:weakauras".to_string(),
            website_url: Some("https://example.com".to_string()),
            provider_project_id: Some(123),
            provider_file_id: Some(456),
            download_count: 999,
        }],
        failures: Vec::new(),
    });

    assert!(rendered.contains("Query: weakauras"));
    assert!(rendered.contains("Found 1 result(s):"));
    assert!(rendered.contains("WeakAuras | provider: curseforge"));
    assert!(rendered.contains("summary: Aura tracking"));
}

#[test]
fn render_addon_search_catalog_lists_partial_failures() {
    let rendered = render_addon_search_catalog(&AddonSearchCatalogResult {
        query: "weakauras".to_string(),
        provider_id: Some("curseforge".to_string()),
        result_count: 0,
        failure_count: 1,
        results: Vec::new(),
        failures: vec![AddonSearchProviderFailureResult {
            provider_id: "curseforge".to_string(),
            provider_name: "CurseForge".to_string(),
            source_family: "curseforge_mod".to_string(),
            message: "fixture failure".to_string(),
        }],
    });

    assert!(rendered.contains("Provider: curseforge"));
    assert!(rendered.contains("No addons found."));
    assert!(rendered.contains("Provider failures: 1"));
    assert!(rendered.contains("CurseForge (curseforge)"));
    assert!(rendered.contains("fixture failure"));
}

#[test]
fn render_addon_inventory_reports_tracked_and_untracked_addons() {
    let rendered = render_addon_inventory(&AddonInventoryResult {
        target_addon_root: PathBuf::from("Interface/AddOns"),
        registry_path: PathBuf::from("addons.toml"),
        tracked_package_count: 1,
        tracked_addon_count: 2,
        tracked_packages: vec![sample_tracked_package("weakauras")],
        untracked_addons: vec!["LooseAddon".to_string()],
    });

    assert!(rendered.contains("Tracked packages: 1"));
    assert!(rendered.contains("Tracked addons: 2"));
    assert!(rendered.contains("weakauras => local.zip [WeakAuras, WeakAurasOptions]"));
    assert!(rendered.contains("Untracked addon directories: LooseAddon"));
}

#[test]
fn render_addon_adopt_reports_snapshot_archive() {
    let rendered = render_addon_adopt(&AdoptedAddonPackageResult {
        dry_run: false,
        source: sample_source(),
        source_label: "local.zip".to_string(),
        package_id: "guild-ui".to_string(),
        addon_count: 2,
        addons: vec![
            sample_tracked_addon("WeakAuras"),
            sample_tracked_addon("SharedMedia"),
        ],
        registry_path: PathBuf::from("app-data/wow/test-install/retail/addons/addons.toml"),
    });

    assert!(rendered.contains("Adopted package: guild-ui"));
    assert!(rendered.contains("Snapshot archive: local.zip"));
    assert!(rendered.contains("Addons: WeakAuras, SharedMedia"));
    assert!(rendered.contains("Registry: app-data/wow/test-install/retail/addons/addons.toml"));
}

#[test]
fn render_addon_relink_reports_source_transition() {
    let rendered = render_addon_relink(&RelinkedAddonPackageResult {
        dry_run: true,
        package_id: "plater".to_string(),
        previous_source: sample_source(),
        previous_source_label: "local.zip".to_string(),
        source: crate::core::app::AddonSourceResult {
            kind: crate::core::app::AddonSourceKindResult::GitHubRelease,
            display_name: "github:foo/plater".to_string(),
            dependency_resolution_capability:
                crate::core::app::AddonDependencyResolutionCapabilityValue::Unsupported,
            local_archive_path: None,
            url: None,
            mod_id: None,
            file_id: None,
            owner: Some("foo".to_string()),
            repo: Some("plater".to_string()),
            tag: None,
            asset_name: None,
            project_id: None,
            release_id: None,
            slug: None,
            version: None,
        },
        source_label: "github:foo/plater".to_string(),
        addon_count: 1,
        addons: vec![sample_tracked_addon("Plater")],
        registry_path: PathBuf::from("addon-registry.json"),
        cleared_metadata: true,
    });

    assert!(rendered.contains("Dry run only."));
    assert!(rendered.contains("Package: plater"));
    assert!(rendered.contains("From: local.zip"));
    assert!(rendered.contains("To: github:foo/plater"));
    assert!(rendered.contains("Metadata: cleared"));
}

#[test]
fn render_addon_install_reports_written_files() {
    let rendered = render_addon_install(&InstalledAddonPackageResult {
        dry_run: false,
        source: sample_source(),
        source_label: "local.zip".to_string(),
        package_id: "weakauras".to_string(),
        addon_count: 2,
        addons: vec![
            sample_tracked_addon("WeakAuras"),
            sample_tracked_addon("WeakAurasOptions"),
        ],
        files_to_write: 0,
        written_files: 20,
        replaced_addon_count: 1,
        replaced_addons: vec!["OldWeakAuras".to_string()],
        registry_path: PathBuf::from("addons.toml"),
        backup_path: Some(PathBuf::from("backup.zip")),
    });

    assert!(rendered.contains("Installed package: weakauras"));
    assert!(rendered.contains("Addons: WeakAuras, WeakAurasOptions"));
    assert!(rendered.contains("Replaced addons: OldWeakAuras"));
    assert!(rendered.contains("Backup: backup.zip"));
}

#[test]
fn render_addon_update_reports_package_summaries() {
    let rendered = render_addon_update(&UpdatedAddonPackageResult {
        dry_run: false,
        registry_path: PathBuf::from("addons.toml"),
        files_to_write: 0,
        written_files: 12,
        updated_package_count: 1,
        updated_packages: vec![sample_tracked_package("weakauras")],
        installed_dependency_package_count: 1,
        installed_dependency_packages: vec![sample_tracked_package("sharedmedia")],
        ignored_package_count: 1,
        ignored_packages: vec!["details".to_string()],
        backup_path: None,
    });

    assert!(rendered.contains("Updated packages: weakauras [WeakAuras, WeakAurasOptions]"));
    assert!(
        rendered
            .contains("Installed dependency packages: sharedmedia [WeakAuras, WeakAurasOptions]")
    );
    assert!(rendered.contains("Ignored packages: details"));
    assert!(rendered.contains("Written files: 12"));
    assert!(rendered.contains("Backup: none"));
}

#[test]
fn render_addon_remove_reports_registry_cleanup() {
    let rendered = render_addon_remove(&RemovedAddonPackageResult {
        dry_run: false,
        registry_path: PathBuf::from("addons.toml"),
        removed_package_count: 1,
        removed_packages: vec![sample_tracked_package("weakauras")],
        removed_addon_count: 2,
        removed_addons: vec!["WeakAuras".to_string(), "WeakAurasOptions".to_string()],
        registry_cleaned: true,
        backup_path: None,
    });

    assert!(rendered.contains("Removed packages: weakauras"));
    assert!(rendered.contains("Removed addon directories: WeakAuras, WeakAurasOptions"));
    assert!(rendered.contains("Registry cleaned: true"));
}
