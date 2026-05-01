use super::*;

#[test]
fn render_addon_index_inspection_lists_packages() {
    let rendered = render_addon_index_inspection(&AddonIndexInspectionResult {
        index_path: PathBuf::from("addons.toml"),
        name: "Curated".to_string(),
        description: Some("test".to_string()),
        package_count: 2,
        identity_hint_coverage: AddonIndexIdentityHintCoverageResult {
            package_count_with_both_exact_hints: 0,
            package_count_with_any_exact_hints: 1,
            package_count_with_match_package_ids: 1,
            package_count_with_addon_directories: 0,
            package_count_without_match_package_ids: 1,
            package_count_without_addon_directories: 2,
            package_count_without_exact_hints: 1,
            packages_without_match_package_ids: vec!["weakauras".to_string()],
            packages_without_addon_directories: vec![
                "details".to_string(),
                "weakauras".to_string(),
            ],
            packages_without_exact_hints: vec!["weakauras".to_string()],
        },
        warning_count: 2,
        blocking_warning_count: 1,
        advisory_warning_count: 1,
        warnings: vec![
            AddonIndexInspectionWarningResult {
                code: AddonIndexInspectionWarningCodeResult::MissingAddonDirectories,
                severity: AddonIndexInspectionWarningSeverityResult::Advisory,
                package_id: "details".to_string(),
                message: "package `details` does not declare addon_directories".to_string(),
            },
            AddonIndexInspectionWarningResult {
                code: AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints,
                severity: AddonIndexInspectionWarningSeverityResult::Blocking,
                package_id: "weakauras".to_string(),
                message: "package `weakauras` does not declare exact identity hints".to_string(),
            },
        ],
        packages: vec![
            sample_index_package("details", "2.0.0"),
            sample_index_package("weakauras", "5.18.2"),
        ],
    });

    assert!(rendered.contains("Index: addons.toml"));
    assert!(rendered.contains("Name: Curated"));
    assert!(rendered.contains("Packages: 2"));
    assert!(rendered.contains("Exact identity hints: 1/2"));
    assert!(rendered.contains("Both exact hints: 0"));
    assert!(rendered.contains("match_package_ids hints: 1"));
    assert!(rendered.contains("addon_directories hints: 0"));
    assert!(rendered.contains("Missing match_package_ids: 1"));
    assert!(rendered.contains("Missing addon_directories: 2"));
    assert!(rendered.contains("Packages without exact identity hints: weakauras"));
    assert!(rendered.contains("Blocking warnings: 1"));
    assert!(rendered.contains("Advisory warnings: 1"));
    assert!(rendered.contains("Warnings: 2"));
    assert!(rendered.contains("advisory missing_addon_directories [details]"));
    assert!(rendered.contains("blocking missing_exact_identity_hints [weakauras]"));
    assert!(rendered.contains("details 2.0.0 => local.zip"));
    assert!(rendered.contains("weakauras 5.18.2 => local.zip"));
}

#[test]
fn render_addon_index_install_reports_dry_run_summary() {
    let rendered = render_addon_index_install(&AddonIndexInstallResult {
        index_path: PathBuf::from("addons.toml"),
        package: sample_index_package("details", "2.0.0"),
        install: InstalledAddonPackageResult {
            dry_run: true,
            source: sample_source(),
            source_label: "local.zip".to_string(),
            package_id: "details".to_string(),
            addon_count: 2,
            addons: vec![
                sample_tracked_addon("Details"),
                sample_tracked_addon("Details_Streamer"),
            ],
            files_to_write: 18,
            written_files: 0,
            replaced_addon_count: 0,
            replaced_addons: Vec::new(),
            registry_path: PathBuf::from("addon-registry.json"),
            backup_path: Some(PathBuf::from("backup.zip")),
        },
    });

    assert!(rendered.contains("Dry run only."));
    assert!(rendered.contains("Package: details 2.0.0"));
    assert!(rendered.contains("Addons: Details, Details_Streamer"));
    assert!(rendered.contains("Files to write: 18"));
    assert!(rendered.contains("Backup: backup.zip"));
}

#[test]
fn render_addon_index_relink_reports_source_and_metadata_changes() {
    let rendered = render_addon_index_relink(&AddonIndexRelinkResult {
        index_path: PathBuf::from("addons.toml"),
        package: sample_index_package("details", "2.0.0"),
        dry_run: true,
        tracked_package_id: "details-local".to_string(),
        previous_source: sample_source(),
        previous_source_label: "local.zip".to_string(),
        source: crate::core::app::AddonSourceResult {
            kind: crate::core::app::AddonSourceKindResult::HttpArchive,
            display_name: "https://example.invalid/details.zip".to_string(),
            dependency_resolution_capability:
                crate::core::app::AddonDependencyResolutionCapabilityValue::Unsupported,
            local_archive_path: None,
            url: Some("https://example.invalid/details.zip".to_string()),
            mod_id: None,
            file_id: None,
            owner: None,
            repo: None,
            tag: None,
            asset_name: None,
        },
        source_label: "https://example.invalid/details.zip".to_string(),
        addon_count: 1,
        addons: vec![sample_tracked_addon("Details")],
        metadata: crate::core::app::AddonPackageMetadataValue {
            index_name: Some("Fixture Index".to_string()),
            index_package_id: Some("details".to_string()),
            package_name: Some("Details".to_string()),
            version: Some("2.0.0".to_string()),
            source_url: Some("https://example.invalid/details.zip".to_string()),
            website_url: None,
            source_sha256: None,
            supported_flavors: vec!["retail".to_string()],
        },
        registry_path: PathBuf::from("addon-registry.json"),
        source_changed: true,
        metadata_changed: true,
    });

    assert!(rendered.contains("Dry run only."));
    assert!(rendered.contains("Index package: details 2.0.0"));
    assert!(rendered.contains("Tracked package: details-local"));
    assert!(rendered.contains("From: local.zip"));
    assert!(rendered.contains("To: https://example.invalid/details.zip"));
    assert!(rendered.contains("Source changed: true"));
    assert!(rendered.contains("Metadata changed: true"));
}

#[test]
fn render_addon_index_validation_reports_invalid_result() {
    let rendered = render_addon_index_validation(&AddonIndexValidationResult {
        index_path: PathBuf::from("addons.toml"),
        name: "Curated".to_string(),
        package_count: 2,
        identity_hint_coverage: AddonIndexIdentityHintCoverageResult {
            package_count_with_both_exact_hints: 0,
            package_count_with_any_exact_hints: 1,
            package_count_with_match_package_ids: 1,
            package_count_with_addon_directories: 0,
            package_count_without_match_package_ids: 1,
            package_count_without_addon_directories: 2,
            package_count_without_exact_hints: 1,
            packages_without_match_package_ids: vec!["weakauras".to_string()],
            packages_without_addon_directories: vec![
                "details".to_string(),
                "weakauras".to_string(),
            ],
            packages_without_exact_hints: vec!["weakauras".to_string()],
        },
        valid: false,
        warning_count: 2,
        blocking_warning_count: 1,
        advisory_warning_count: 1,
        warnings: vec![
            AddonIndexInspectionWarningResult {
                code: AddonIndexInspectionWarningCodeResult::MissingAddonDirectories,
                severity: AddonIndexInspectionWarningSeverityResult::Advisory,
                package_id: "details".to_string(),
                message: "package `details` does not declare addon_directories".to_string(),
            },
            AddonIndexInspectionWarningResult {
                code: AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints,
                severity: AddonIndexInspectionWarningSeverityResult::Blocking,
                package_id: "weakauras".to_string(),
                message: "package `weakauras` does not declare exact identity hints".to_string(),
            },
        ],
    });

    assert!(rendered.contains("Status: invalid"));
    assert!(rendered.contains("Valid: false"));
    assert!(rendered.contains("Index: addons.toml"));
    assert!(rendered.contains("Both exact hints: 0"));
    assert!(rendered.contains("Blocking warnings: 1"));
    assert!(rendered.contains("Advisory warnings: 1"));
    assert!(rendered.contains("Warnings: 2"));
    assert!(rendered.contains("blocking missing_exact_identity_hints [weakauras]"));
}

#[test]
fn render_addon_index_validation_reports_valid_status_with_advisory_warnings() {
    let rendered = render_addon_index_validation(&AddonIndexValidationResult {
        index_path: PathBuf::from("addons.toml"),
        name: "Curated".to_string(),
        package_count: 1,
        identity_hint_coverage: AddonIndexIdentityHintCoverageResult {
            package_count_with_both_exact_hints: 0,
            package_count_with_any_exact_hints: 1,
            package_count_with_match_package_ids: 1,
            package_count_with_addon_directories: 0,
            package_count_without_match_package_ids: 0,
            package_count_without_addon_directories: 1,
            package_count_without_exact_hints: 0,
            packages_without_match_package_ids: Vec::new(),
            packages_without_addon_directories: vec!["details".to_string()],
            packages_without_exact_hints: Vec::new(),
        },
        valid: true,
        warning_count: 1,
        blocking_warning_count: 0,
        advisory_warning_count: 1,
        warnings: vec![AddonIndexInspectionWarningResult {
            code: AddonIndexInspectionWarningCodeResult::MissingAddonDirectories,
            severity: AddonIndexInspectionWarningSeverityResult::Advisory,
            package_id: "details".to_string(),
            message: "package `details` does not declare addon_directories".to_string(),
        }],
    });

    assert!(rendered.contains("Status: valid"));
    assert!(rendered.contains("Valid: true"));
    assert!(rendered.contains("Blocking warnings: 0"));
    assert!(rendered.contains("Advisory warnings: 1"));
    assert!(rendered.contains("advisory missing_addon_directories [details]"));
}

#[test]
fn render_addon_index_suggestion_reports_match_strategies_and_hint_additions() {
    let rendered = render_addon_index_suggestion(&AddonIndexSuggestionResult {
        index_path: PathBuf::from("addons.toml"),
        index_name: "Curated".to_string(),
        index_package_count: 3,
        considered_package_count: 2,
        suggested_package_count: 1,
        complete_package_count: 0,
        no_match_package_count: 1,
        ambiguous_match_package_count: 0,
        skipped_unsupported_flavor_package_count: 1,
        packages: vec![
            AddonIndexPackageSuggestionResult {
                package_id: "curated-plater".to_string(),
                package_name: "Curated Plater".to_string(),
                current_match_package_ids: Vec::new(),
                current_addon_directories: Vec::new(),
                status: AddonIndexPackageSuggestionStatusResult::Suggested,
                matched_tracked_package_id: Some("plater".to_string()),
                match_strategy: Some(AddonIndexTrackedMatchStrategyResult::SourceFamilyIdentity),
                matched_addon_directories: vec!["Plater".to_string()],
                match_package_ids_to_add: vec!["plater".to_string()],
                addon_directories_to_add: vec!["Plater".to_string()],
                message: "matched tracked package `plater` by source family identity".to_string(),
            },
            AddonIndexPackageSuggestionResult {
                package_id: "weakauras".to_string(),
                package_name: "WeakAuras".to_string(),
                current_match_package_ids: Vec::new(),
                current_addon_directories: Vec::new(),
                status: AddonIndexPackageSuggestionStatusResult::NoLocalMatch,
                matched_tracked_package_id: None,
                match_strategy: None,
                matched_addon_directories: Vec::new(),
                match_package_ids_to_add: Vec::new(),
                addon_directories_to_add: Vec::new(),
                message:
                    "no tracked addon package from the current registry matched this index package"
                        .to_string(),
            },
        ],
    });

    assert!(rendered.contains("Index: addons.toml"));
    assert!(rendered.contains("Name: Curated"));
    assert!(rendered.contains("Index packages: 3"));
    assert!(rendered.contains("Considered packages: 2"));
    assert!(rendered.contains("Suggested packages: 1"));
    assert!(rendered.contains("No local match packages: 1"));
    assert!(rendered.contains("Skipped unsupported flavor packages: 1"));
    assert!(rendered.contains("- curated-plater (suggested)"));
    assert!(rendered.contains("matched tracked package: plater (source_family_identity)"));
    assert!(rendered.contains("match_package_ids to add: plater"));
    assert!(rendered.contains("addon_directories to add: Plater"));
    assert!(rendered.contains("- weakauras (no_local_match)"));
}

#[test]
fn render_addon_index_attach_reports_blocked_and_planned_packages() {
    let rendered = render_addon_index_attach(&AddonIndexAttachResult {
        index_path: PathBuf::from("addons.toml"),
        index_name: "Curated".to_string(),
        dry_run: true,
        ready: false,
        applied: false,
        partial_apply: false,
        registry_path: PathBuf::from("addon-registry.json"),
        index_package_count: 3,
        considered_package_count: 2,
        change_package_count: 1,
        attached_package_count: 0,
        already_attached_package_count: 0,
        blocked_package_count: 1,
        skipped_unsupported_flavor_package_count: 1,
        packages: vec![
            AddonIndexAttachPackageResult {
                package: sample_index_package("curated-plater", "2.0.0"),
                status: AddonIndexAttachPackageStatusResult::WouldAttach,
                matched_tracked_package_id: Some("plater".to_string()),
                match_strategy: Some(
                    AddonIndexTrackedMatchStrategyResult::SourceFamilyIdentity,
                ),
                previous_source: Some(sample_source()),
                previous_source_label: Some("local.zip".to_string()),
                source: Some(crate::core::app::AddonSourceResult {
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
                }),
                source_label: Some("github:foo/plater".to_string()),
                source_changed: true,
                metadata_changed: true,
                message:
                    "matched tracked package `plater` by source family identity; would attach curated source and metadata"
                        .to_string(),
            },
            AddonIndexAttachPackageResult {
                package: sample_index_package("weakauras", "5.18.2"),
                status: AddonIndexAttachPackageStatusResult::NoLocalMatch,
                matched_tracked_package_id: None,
                match_strategy: None,
                previous_source: None,
                previous_source_label: None,
                source: None,
                source_label: None,
                source_changed: false,
                metadata_changed: false,
                message:
                    "no tracked addon package from the current registry matched this index package"
                        .to_string(),
            },
        ],
    });

    assert!(rendered.contains("Status: blocked"));
    assert!(rendered.contains("Dry run: true"));
    assert!(rendered.contains("Ready: false"));
    assert!(rendered.contains("Applied: false"));
    assert!(rendered.contains("Partial apply: false"));
    assert!(rendered.contains("Planned changes: 1"));
    assert!(rendered.contains("Blocked packages: 1"));
    assert!(rendered.contains("Skipped unsupported flavor packages: 1"));
    assert!(rendered.contains("- curated-plater 2.0.0 (would_attach)"));
    assert!(rendered.contains("tracked package: plater (source_family_identity)"));
    assert!(rendered.contains("from: local.zip"));
    assert!(rendered.contains("to: github:foo/plater"));
    assert!(rendered.contains("source changed: true"));
    assert!(rendered.contains("metadata changed: true"));
    assert!(rendered.contains("- weakauras 5.18.2 (no_local_match)"));
}

#[test]
fn render_addon_index_scaffold_reports_summary_counts() {
    let rendered = render_addon_index_scaffold(&AddonIndexScaffoldResult {
        index_path: PathBuf::from("addons.toml"),
        index_name: "Guild UI".to_string(),
        package_count: 2,
        used_metadata_package_count: 1,
        inferred_name_package_count: 1,
        inferred_version_package_count: 2,
        placeholder_version_package_count: 1,
        package_ids: vec!["plater".to_string(), "weakauras".to_string()],
    });

    assert!(rendered.contains("Wrote addon index scaffold: addons.toml"));
    assert!(rendered.contains("Name: Guild UI"));
    assert!(rendered.contains("Packages: 2"));
    assert!(rendered.contains("Used existing metadata: 1"));
    assert!(rendered.contains("Inferred names: 1"));
    assert!(rendered.contains("Inferred versions: 2"));
    assert!(rendered.contains("Placeholder versions: 1"));
    assert!(rendered.contains("Package ids: plater, weakauras"));
}

#[test]
fn render_addon_index_update_reports_written_files() {
    let rendered = render_addon_index_update(&AddonIndexUpdateResult {
        index_path: PathBuf::from("addons.toml"),
        selected_package_count: 2,
        selected_packages: vec![
            sample_index_package("details", "2.0.0"),
            sample_index_package("weakauras", "5.18.2"),
        ],
        update: UpdatedAddonPackageResult {
            dry_run: false,
            registry_path: PathBuf::from("addon-registry.json"),
            files_to_write: 0,
            written_files: 24,
            updated_package_count: 2,
            updated_packages: Vec::new(),
            installed_dependency_package_count: 1,
            installed_dependency_packages: vec![sample_tracked_package("sharedmedia")],
            ignored_package_count: 1,
            ignored_packages: vec!["plater".to_string()],
            backup_path: None,
        },
    });

    assert!(rendered.contains("Updated index packages: details 2.0.0, weakauras 5.18.2"));
    assert!(
        rendered
            .contains("Installed dependency packages: sharedmedia [WeakAuras, WeakAurasOptions]")
    );
    assert!(rendered.contains("Ignored packages: plater"));
    assert!(rendered.contains("Index: addons.toml"));
    assert!(rendered.contains("Written files: 24"));
    assert!(rendered.contains("Backup: none"));
}
