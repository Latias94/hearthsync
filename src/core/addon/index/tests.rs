use std::cell::{Cell, RefCell};
use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::tempdir;
use zip::CompressionMethod;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::{
    AddonIndexAttachPackageStatus, AddonIndexAttachRequest, AddonIndexInstallRequest,
    AddonIndexRelinkRequest, AddonIndexScaffoldRequest, AddonIndexSuggestionRequest,
    attach_addons_from_index, attach_addons_from_index_task, inspect_addon_index,
    install_addon_from_index, install_addon_from_index_task, relink_addon_from_index,
    relink_addon_from_index_task, scaffold_addon_index, suggest_addon_index_hints,
    update_addons_from_index, update_addons_from_index_task,
    update_addons_from_index_task_with_provider,
};
use crate::core::addon::index::AddonIndexUpdateRequest;
use crate::core::addon::provider::ResolveAddonDependenciesRequest;
use crate::core::addon::{
    AddonDependencyResolutionCapability, AddonProvider,
    AddonSearchRequest as ProviderAddonSearchRequest, AddonSearchResult, AddonSourceRef,
    AddonSourceResolutionPolicy, InstallAddonRequest, MaterializeSourceInputRequest,
    MaterializeSourceRefRequest, MaterializedAddonSource, ResolvedAddonDependencies,
    canonicalize_local_archive_path, install_addon, install_addon_task_with_provider, list_addons,
    policy::{AddonReleaseChannel, SetAddonPolicyRequest, set_addon_policy},
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};
use crate::core::task::{
    NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressEvent, VecTaskProgressSink,
};

fn addon_state_paths(
    installation: &DetectedFlavorInstallation,
) -> crate::core::addon::AddonStatePaths {
    crate::core::addon::AddonStatePaths::for_installation(
        crate::core::addon::AddonStateStorageKind::default(),
        installation,
    )
    .expect("addon state paths")
}

#[test]
fn inspect_addon_index_reads_packages() {
    let temp = tempdir().expect("temp dir");
    let archive_path = temp.path().join("details.zip");
    let index_path = write_index(temp.path(), &archive_path);

    let inspection = inspect_addon_index(&index_path).expect("inspect index");

    assert_eq!(inspection.index.name, "Fixture Index");
    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.index.packages[0].id, "details");
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_both_exact_hints,
        0
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_any_exact_hints,
        0
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_match_package_ids,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_addon_directories,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_exact_hints,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .packages_without_match_package_ids,
        vec!["details".to_string()]
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .packages_without_addon_directories,
        vec!["details".to_string()]
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .packages_without_exact_hints,
        vec!["details".to_string()]
    );
    assert_eq!(inspection.warning_count, 1);
    assert_eq!(inspection.blocking_warning_count, 1);
    assert_eq!(inspection.advisory_warning_count, 0);
    assert_eq!(inspection.warnings.len(), 1);
    assert!(matches!(
        inspection.warnings[0].code,
        super::AddonIndexInspectionWarningCode::MissingExactIdentityHints
    ));
    assert!(matches!(
        inspection.warnings[0].severity,
        super::AddonIndexInspectionWarningSeverity::Blocking
    ));
    assert_eq!(inspection.warnings[0].package_id, "details");
    assert!(
        inspection.warnings[0]
            .message
            .contains("does not declare exact identity hints")
    );
}

#[test]
fn inspect_addon_index_rejects_duplicate_match_package_ids() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "details"
name = "Details"
version = "1.0.0"
match_package_ids = ["legacy-details", "LEGACY-DETAILS"]
source = { kind = "http_archive", url = "https://example.invalid/details.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let error = inspect_addon_index(&index_path).expect_err("duplicate hint should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("duplicate match package id"));
}

#[test]
fn inspect_addon_index_rejects_blank_addon_directories() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "details"
name = "Details"
version = "1.0.0"
addon_directories = [""]
source = { kind = "http_archive", url = "https://example.invalid/details.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let error = inspect_addon_index(&index_path).expect_err("blank addon directory should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("invalid addon directory name"));
    assert!(error.to_string().contains("for package `details`"));
}

#[test]
fn inspect_addon_index_rejects_non_portable_addon_directories() {
    for addon_directory in ["Bad/Addon", "CON", "Weak:Auras"] {
        let temp = tempdir().expect("temp dir");
        let index_path = temp.path().join("index.toml");
        fs::write(
            &index_path,
            format!(
                r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "details"
name = "Details"
version = "1.0.0"
addon_directories = ["{addon_directory}"]
source = {{ kind = "http_archive", url = "https://example.invalid/details.zip" }}
supported_flavors = ["retail"]
"#
            ),
        )
        .expect("write index");

        let error =
            inspect_addon_index(&index_path).expect_err("non-portable addon directory should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(error.to_string().contains("invalid addon directory name"));
        assert!(error.to_string().contains("for package `details`"));
    }
}

#[test]
fn inspect_addon_index_rejects_duplicate_addon_directories() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "details"
name = "Details"
version = "1.0.0"
addon_directories = ["Details", "details"]
source = { kind = "http_archive", url = "https://example.invalid/details.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let error =
        inspect_addon_index(&index_path).expect_err("duplicate addon directory should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("duplicate addon directory"));
}

#[test]
fn inspect_addon_index_rejects_case_insensitive_duplicate_package_ids() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "Details"
name = "Details"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/details.zip" }
supported_flavors = ["retail"]

[[packages]]
id = "details"
name = "Details Copy"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/details-copy.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let error = inspect_addon_index(&index_path).expect_err("duplicate package id should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(
        error
            .to_string()
            .contains("duplicate addon index package id")
    );
}

#[test]
fn inspect_addon_index_summarizes_exact_identity_hint_coverage() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "legacy-bridge"
name = "Legacy Bridge"
version = "1.0.0"
match_package_ids = ["legacy-package"]
source = { kind = "http_archive", url = "https://example.invalid/legacy.zip" }
supported_flavors = ["retail"]

[[packages]]
id = "directory-bridge"
name = "Directory Bridge"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/directories.zip" }
addon_directories = ["DirectoryBridge"]
supported_flavors = ["retail"]

[[packages]]
id = "no-bridge"
name = "No Bridge"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/no-bridge.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let inspection = inspect_addon_index(&index_path).expect("inspect index");

    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_both_exact_hints,
        0
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_any_exact_hints,
        2
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_match_package_ids,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_with_addon_directories,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_match_package_ids,
        2
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_addon_directories,
        2
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_exact_hints,
        1
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .packages_without_match_package_ids,
        vec!["directory-bridge".to_string(), "no-bridge".to_string()]
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .packages_without_addon_directories,
        vec!["legacy-bridge".to_string(), "no-bridge".to_string()]
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .packages_without_exact_hints,
        vec!["no-bridge".to_string()]
    );
    assert_eq!(inspection.warning_count, 3);
    assert_eq!(inspection.blocking_warning_count, 1);
    assert_eq!(inspection.advisory_warning_count, 2);
    assert_eq!(inspection.warnings.len(), 3);
    assert!(inspection.warnings.iter().any(|warning| {
        matches!(
            warning.code,
            super::AddonIndexInspectionWarningCode::MissingMatchPackageIds
        ) && matches!(
            warning.severity,
            super::AddonIndexInspectionWarningSeverity::Advisory
        ) && warning.package_id == "directory-bridge"
    }));
    assert!(inspection.warnings.iter().any(|warning| {
        matches!(
            warning.code,
            super::AddonIndexInspectionWarningCode::MissingAddonDirectories
        ) && matches!(
            warning.severity,
            super::AddonIndexInspectionWarningSeverity::Advisory
        ) && warning.package_id == "legacy-bridge"
    }));
    assert!(inspection.warnings.iter().any(|warning| {
        matches!(
            warning.code,
            super::AddonIndexInspectionWarningCode::MissingExactIdentityHints
        ) && matches!(
            warning.severity,
            super::AddonIndexInspectionWarningSeverity::Blocking
        ) && warning.package_id == "no-bridge"
    }));
}

#[test]
fn suggest_addon_index_hints_reports_missing_exact_hints_from_local_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(inventory.tracked_packages[0].package_id, "plater");

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "classic-only"
name = "Classic Only"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/classic.zip" }}
supported_flavors = ["classic"]

[[packages]]
id = "unknown-addon"
name = "Unknown Addon"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/unknown.zip" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\"),
        ),
    )
    .expect("write index");

    let suggestion = suggest_addon_index_hints(AddonIndexSuggestionRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        index_path,
        name: None,
    })
    .expect("suggest addon index hints");

    assert_eq!(suggestion.index_package_count, 3);
    assert_eq!(suggestion.considered_package_count, 2);
    assert_eq!(suggestion.suggested_package_count, 1);
    assert_eq!(suggestion.complete_package_count, 0);
    assert_eq!(suggestion.no_match_package_count, 1);
    assert_eq!(suggestion.ambiguous_match_package_count, 0);
    assert_eq!(suggestion.skipped_unsupported_flavor_package_count, 1);

    let curated = suggestion
        .packages
        .iter()
        .find(|package| package.package_id == "curated-plater")
        .expect("curated package");
    assert!(matches!(
        curated.status,
        super::AddonIndexPackageSuggestionStatus::Suggested
    ));
    assert_eq!(
        curated.matched_tracked_package_id.as_deref(),
        Some("plater")
    );
    assert!(matches!(
        curated.match_strategy,
        Some(super::AddonIndexTrackedMatchStrategy::SourceIdentity)
    ));
    assert_eq!(curated.match_package_ids_to_add, vec!["plater".to_string()]);
    assert_eq!(curated.addon_directories_to_add, vec!["Plater".to_string()]);

    let unknown = suggestion
        .packages
        .iter()
        .find(|package| package.package_id == "unknown-addon")
        .expect("unknown package");
    assert!(matches!(
        unknown.status,
        super::AddonIndexPackageSuggestionStatus::NoLocalMatch
    ));
}

#[test]
fn suggest_addon_index_hints_marks_complete_packages_when_local_hints_already_exist() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "plater"
name = "Plater"
version = "2.0.0"
addon_directories = ["Plater"]
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\"),
        ),
    )
    .expect("write index");

    let suggestion = suggest_addon_index_hints(AddonIndexSuggestionRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        index_path,
        name: Some("plater".to_string()),
    })
    .expect("suggest addon index hints");

    assert_eq!(suggestion.considered_package_count, 1);
    assert_eq!(suggestion.complete_package_count, 1);
    assert_eq!(suggestion.suggested_package_count, 0);
    let package = &suggestion.packages[0];
    assert!(matches!(
        package.status,
        super::AddonIndexPackageSuggestionStatus::Complete
    ));
    assert_eq!(package.match_package_ids_to_add, Vec::<String>::new());
    assert_eq!(package.addon_directories_to_add, Vec::<String>::new());
    assert!(matches!(
        package.match_strategy,
        Some(super::AddonIndexTrackedMatchStrategy::ExactPackageId)
    ));
}

#[test]
fn suggest_addon_index_hints_surfaces_ambiguous_local_matches_without_failing_whole_run() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let alpha_archive_path = temp.path().join("PlaterAlpha.zip");
    let beta_archive_path = temp.path().join("PlaterBeta.zip");
    create_addon_archive(
        &alpha_archive_path,
        &[(
            "PlaterAlpha/PlaterAlpha.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &beta_archive_path,
        &[(
            "PlaterBeta/PlaterBeta.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    for archive_path in [&alpha_archive_path, &beta_archive_path] {
        install_addon(InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");
    }

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Plater"
version = "2.0.0"
source = { kind = "http_archive", url = "https://example.invalid/plater.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let suggestion = suggest_addon_index_hints(AddonIndexSuggestionRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        index_path,
        name: None,
    })
    .expect("suggest addon index hints");

    assert_eq!(suggestion.considered_package_count, 1);
    assert_eq!(suggestion.ambiguous_match_package_count, 1);
    let package = &suggestion.packages[0];
    assert!(matches!(
        package.status,
        super::AddonIndexPackageSuggestionStatus::AmbiguousLocalMatch
    ));
    assert!(
        package
            .message
            .contains("matched multiple tracked packages")
    );
}

#[test]
fn scaffold_addon_index_writes_index_from_tracked_registry_and_metadata() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: Some(crate::core::addon::AddonPackageMetadata {
            index_name: Some("Legacy Index".to_string()),
            index_package_id: Some("curated-plater".to_string()),
            package_name: Some("Curated Plater".to_string()),
            version: Some("2.0.0".to_string()),
            source_url: Some("https://example.invalid/plater.zip".to_string()),
            website_url: Some("https://example.invalid".to_string()),
            source_sha256: Some("deadbeef".to_string()),
            supported_flavors: vec!["retail".to_string()],
        }),
    })
    .expect("install tracked addon");

    let index_path = temp.path().join("generated-index.toml");
    let result = scaffold_addon_index(AddonIndexScaffoldRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        index_path: index_path.clone(),
        index_name: "Guild UI".to_string(),
        description: Some("Scaffolded".to_string()),
        name: None,
        overwrite: false,
    })
    .expect("scaffold addon index");

    assert_eq!(result.index_path, index_path);
    assert_eq!(result.index_name, "Guild UI");
    assert_eq!(result.package_count, 1);
    assert_eq!(result.used_metadata_package_count, 1);
    assert_eq!(result.inferred_name_package_count, 0);
    assert_eq!(result.inferred_version_package_count, 0);
    assert_eq!(result.placeholder_version_package_count, 0);
    assert_eq!(result.package_ids, vec!["curated-plater".to_string()]);

    let inspection = inspect_addon_index(&result.index_path).expect("inspect scaffolded index");
    assert_eq!(inspection.index.name, "Guild UI");
    assert_eq!(inspection.index.description.as_deref(), Some("Scaffolded"));
    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.index.packages[0].id, "curated-plater");
    assert_eq!(
        inspection.index.packages[0].match_package_ids,
        vec!["plater".to_string()]
    );
    assert_eq!(
        inspection.index.packages[0].addon_directories,
        vec!["Plater".to_string()]
    );
    assert_eq!(inspection.index.packages[0].name, "Curated Plater");
    assert_eq!(inspection.index.packages[0].version, "2.0.0");
}

#[test]
fn scaffold_addon_index_rejects_existing_file_without_overwrite() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Title: Details\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let index_path = temp.path().join("generated-index.toml");
    fs::write(&index_path, "schema_version = 1\nname = \"Existing\"\n").expect("existing index");

    let error = scaffold_addon_index(AddonIndexScaffoldRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        index_path,
        index_name: "Guild UI".to_string(),
        description: None,
        name: None,
        overwrite: false,
    })
    .expect_err("existing file should fail without overwrite");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("already exists"));
}

#[test]
fn scaffold_addon_index_without_tracked_registry_mentions_adopt_for_local_addons() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let addon_dir = installation.addon_dir.join("Plater");
    fs::create_dir_all(&addon_dir).expect("plater dir");
    fs::write(
        addon_dir.join("Plater.toc"),
        "## Interface: 110000\n## Title: Plater\n",
    )
    .expect("write toc");

    let error = scaffold_addon_index(AddonIndexScaffoldRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        index_path: temp.path().join("generated-index.toml"),
        index_name: "Guild UI".to_string(),
        description: None,
        name: None,
        overwrite: false,
    })
    .expect_err("missing tracked registry should fail");

    assert!(matches!(error, AppError::Validation(_)));
    let message = error.to_string();
    assert!(message.contains("addon adopt"));
    assert!(message.contains("existing local addons"));
}

#[test]
fn install_addon_from_index_installs_selected_package() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &archive_path);

    let result = install_addon_from_index(AddonIndexInstallRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: "details".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
    })
    .expect("install from index");

    assert_eq!(result.package.id, "details");
    assert!(
        installation
            .addon_dir
            .join("Details")
            .join("Details.toc")
            .exists()
    );
}

#[test]
fn install_addon_from_index_task_reports_index_install_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &archive_path);

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = install_addon_from_index_task(
        AddonIndexInstallRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: "details".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        },
        &cancellation,
        &mut progress,
    )
    .expect("install from index task");

    assert_eq!(result.package.id, "details");
    assert_addon_index_task_progress(
        progress.events(),
        TaskKind::AddonIndexInstall,
        "Installing addon directory",
    );
}

#[test]
fn install_addon_from_index_resolves_relative_local_archive_against_index_path() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_dir = temp.path().join("archives");
    let archive_path = archive_dir.join("details.zip");
    fs::create_dir_all(&archive_dir).expect("archive dir");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = write_index(
        temp.path(),
        Path::new("archives").join("details.zip").as_path(),
    );

    let result = install_addon_from_index(AddonIndexInstallRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: "details".to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
    })
    .expect("install from relative index source");

    assert_eq!(result.package.id, "details");
    assert!(
        installation
            .addon_dir
            .join("Details")
            .join("Details.toc")
            .exists()
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0].source,
        AddonSourceRef::LocalArchive {
            path: normalized_archive_path(&archive_path),
        }
    );
}

#[test]
fn relink_addon_from_index_updates_curated_metadata_without_reinstalling_files() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
source_url = "https://example.invalid/details.zip"
website_url = "https://example.invalid/details"
addon_directories = ["Details"]
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");

    let result = relink_addon_from_index(AddonIndexRelinkRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: "curated-details".to_string(),
        target: Some("details".to_string()),
        dry_run: false,
    })
    .expect("relink from index");

    assert_eq!(result.tracked_package_id, "details");
    assert!(!result.source_changed);
    assert!(result.metadata_changed);
    assert_eq!(
        result.metadata.index_package_id.as_deref(),
        Some("curated-details")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.index_package_id.as_deref()),
        Some("curated-details")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("1.0.0")
    );
}

#[test]
fn relink_addon_from_index_task_reports_relink_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Details"]
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = relink_addon_from_index_task(
        AddonIndexRelinkRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: "curated-details".to_string(),
            target: Some("details".to_string()),
            dry_run: false,
        },
        &cancellation,
        &mut progress,
    )
    .expect("relink from index task");

    assert_eq!(result.tracked_package_id, "details");
    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases.first(),
        Some(&(TaskKind::AddonIndexRelink, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::AddonIndexRelink, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::AddonIndexRelink, TaskPhase::Executing)));
}

#[test]
fn attach_addons_from_index_blocks_without_writing_registry_when_any_package_cannot_attach() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "unknown-addon"
name = "Unknown Addon"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/unknown.zip" }}
supported_flavors = ["retail"]

[[packages]]
id = "classic-only"
name = "Classic Only"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/classic.zip" }}
supported_flavors = ["classic"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    let result = attach_addons_from_index(AddonIndexAttachRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        apply_ready_only: false,
    })
    .expect("attach from index");

    assert!(!result.ready);
    assert!(!result.applied);
    assert_eq!(result.change_package_count, 1);
    assert_eq!(result.attached_package_count, 0);
    assert_eq!(result.blocked_package_count, 1);
    assert_eq!(result.skipped_unsupported_flavor_package_count, 1);
    let curated = result
        .packages
        .iter()
        .find(|package| package.package.id == "curated-plater")
        .expect("curated package");
    assert!(matches!(
        curated.status,
        AddonIndexAttachPackageStatus::WouldAttach
    ));
    assert_eq!(
        curated.matched_tracked_package_id.as_deref(),
        Some("plater")
    );
    assert!(!curated.source_changed);
    assert!(curated.metadata_changed);

    let unknown = result
        .packages
        .iter()
        .find(|package| package.package.id == "unknown-addon")
        .expect("unknown package");
    assert!(matches!(
        unknown.status,
        AddonIndexAttachPackageStatus::NoLocalMatch
    ));

    let classic = result
        .packages
        .iter()
        .find(|package| package.package.id == "classic-only")
        .expect("classic package");
    assert!(matches!(
        classic.status,
        AddonIndexAttachPackageStatus::SkippedUnsupportedFlavor
    ));

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert!(
        inventory.tracked_packages[0].metadata.is_none(),
        "blocked bulk attach must not partially write curated metadata"
    );
}

#[test]
fn attach_addons_from_index_can_apply_ready_packages_when_partial_apply_is_explicit() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]

[[packages]]
id = "unknown-addon"
name = "Unknown Addon"
version = "1.0.0"
source = {{ kind = "http_archive", url = "https://example.invalid/unknown.zip" }}
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    let result = attach_addons_from_index(AddonIndexAttachRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        apply_ready_only: true,
    })
    .expect("attach ready packages from index");

    assert!(!result.ready);
    assert!(result.applied);
    assert!(result.partial_apply);
    assert_eq!(result.change_package_count, 1);
    assert_eq!(result.attached_package_count, 1);
    assert_eq!(result.blocked_package_count, 1);

    let curated = result
        .packages
        .iter()
        .find(|package| package.package.id == "curated-plater")
        .expect("curated package");
    assert!(matches!(
        curated.status,
        AddonIndexAttachPackageStatus::Attached
    ));
    let unknown = result
        .packages
        .iter()
        .find(|package| package.package.id == "unknown-addon")
        .expect("unknown package");
    assert!(matches!(
        unknown.status,
        AddonIndexAttachPackageStatus::NoLocalMatch
    ));

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(
        inventory.tracked_packages[0]
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.index_package_id.as_deref()),
        Some("curated-plater")
    );
}

#[test]
fn attach_addons_from_index_attaches_all_ready_packages_without_reinstalling_files() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let details_archive_path = temp.path().join("Details.zip");
    let plater_archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &details_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &plater_archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    for archive_path in [&details_archive_path, &plater_archive_path] {
        install_addon(InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");
    }

    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Details"]
supported_flavors = ["retail"]

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Plater"]
supported_flavors = ["retail"]
"#,
            details_archive_path
                .display()
                .to_string()
                .replace('\\', "\\\\"),
            plater_archive_path
                .display()
                .to_string()
                .replace('\\', "\\\\"),
        ),
    )
    .expect("write index");

    let result = attach_addons_from_index(AddonIndexAttachRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        apply_ready_only: false,
    })
    .expect("attach from index");

    assert!(result.ready);
    assert!(result.applied);
    assert_eq!(result.change_package_count, 2);
    assert_eq!(result.attached_package_count, 2);
    assert_eq!(result.blocked_package_count, 0);
    assert!(
        result
            .packages
            .iter()
            .all(|package| { matches!(package.status, AddonIndexAttachPackageStatus::Attached) })
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 2);
    assert!(inventory.tracked_packages.iter().any(|package| {
        package.package_id == "details"
            && package
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.index_package_id.as_deref())
                == Some("curated-details")
    }));
    assert!(inventory.tracked_packages.iter().any(|package| {
        package.package_id == "plater"
            && package
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.index_package_id.as_deref())
                == Some("curated-plater")
    }));
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("details toc")
            .contains("1.0.0")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("plater toc")
            .contains("1.0.0")
    );
}

#[test]
fn attach_addons_from_index_task_reports_attach_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_path = temp.path().join("Details.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-details"
name = "Curated Details"
version = "2.0.0"
source = {{ kind = "local_archive", path = "{}" }}
addon_directories = ["Details"]
supported_flavors = ["retail"]
"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write index");

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation),
        installation,
        source: archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = attach_addons_from_index_task(
        AddonIndexAttachRequest {
            state_paths: addon_state_paths(&create_fixture_installation(temp.path())),
            installation: create_fixture_installation(temp.path()),
            index_path,
            name: Some("curated-details".to_string()),
            dry_run: false,
            apply_ready_only: false,
        },
        &cancellation,
        &mut progress,
    )
    .expect("attach from index task");

    assert!(result.applied);
    let phases = progress
        .events()
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();
    assert_eq!(
        phases.first(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Preparing))
    );
    assert_eq!(
        phases.last(),
        Some(&(TaskKind::AddonIndexAttach, TaskPhase::Completed))
    );
    assert!(phases.contains(&(TaskKind::AddonIndexAttach, TaskPhase::Executing)));
}

#[test]
fn update_addons_from_index_uses_index_source_and_skips_unselected_packages() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = temp.path().join("details-updated.zip");
    let extra_archive_path = temp.path().join("omen.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    create_addon_archive(
        &extra_archive_path,
        &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install details");
    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: extra_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install omen");

    let result = update_addons_from_index(AddonIndexUpdateRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update from index");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
    assert!(
        fs::read_to_string(installation.addon_dir.join("Omen").join("Omen.toc"))
            .expect("omen toc")
            .contains("1.0.0")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    let details_package = inventory
        .tracked_packages
        .iter()
        .find(|package| {
            package
                .addons
                .iter()
                .any(|addon| addon.directory_name == "Details")
        })
        .expect("details package");
    assert_eq!(
        details_package.source,
        AddonSourceRef::LocalArchive {
            path: normalized_archive_path(&updated_archive_path),
        }
    );
}

#[test]
fn update_addons_from_index_resolves_relative_local_archive_against_index_path() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let archive_dir = temp.path().join("archives");
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = archive_dir.join("details-updated.zip");
    fs::create_dir_all(&archive_dir).expect("archive dir");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(
        temp.path(),
        Path::new("archives").join("details-updated.zip").as_path(),
    );

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let result = update_addons_from_index(AddonIndexUpdateRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: Some("details".to_string()),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
    })
    .expect("update from relative index source");

    assert_eq!(result.selected_packages.len(), 1);
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("2.0.0")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    let details_package = inventory
        .tracked_packages
        .iter()
        .find(|package| {
            package
                .addons
                .iter()
                .any(|addon| addon.directory_name == "Details")
        })
        .expect("details package");
    assert_eq!(
        details_package.source,
        AddonSourceRef::LocalArchive {
            path: normalized_archive_path(&updated_archive_path),
        }
    );
}

#[test]
fn update_addons_from_index_task_reports_index_update_progress() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = temp.path().join("details-updated.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    let cancellation = NeverCancel;
    let mut progress = VecTaskProgressSink::default();
    let result = update_addons_from_index_task(
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: Some("details".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        },
        &cancellation,
        &mut progress,
    )
    .expect("update from index task");

    assert_eq!(result.selected_packages.len(), 1);
    assert_addon_index_task_progress(
        progress.events(),
        TaskKind::AddonIndexUpdate,
        "Writing updated addon directory",
    );
}

#[test]
fn update_addons_from_index_skips_ignored_tracked_packages_in_bulk_runs() {
    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let installed_archive_path = temp.path().join("details-installed.zip");
    let updated_archive_path = temp.path().join("details-updated.zip");
    create_addon_archive(
        &installed_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 110000\n## Version: 1.0.0\n",
        )],
    );
    create_addon_archive(
        &updated_archive_path,
        &[(
            "Details/Details.toc",
            "## Interface: 120000\n## Version: 2.0.0\n",
        )],
    );
    let index_path = write_index(temp.path(), &updated_archive_path);

    install_addon(InstallAddonRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        source: installed_archive_path.display().to_string(),
        dry_run: false,
        backup_output_path: Some(temp.path().join("backups")),
        replace_existing: false,
        metadata: None,
    })
    .expect("install tracked addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "details-installed".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set ignored policy");

    let result = update_addons_from_index(AddonIndexUpdateRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        index_path,
        name: None,
        dry_run: false,
        backup_output_path: Some(temp.path().join("bulk-backups")),
    })
    .expect("update from index");

    assert!(result.selected_packages.is_empty());
    assert!(result.update.updated_packages.is_empty());
    assert_eq!(
        result.update.ignored_packages,
        vec!["details-installed".to_string()]
    );
    assert!(result.update.backup_path.is_none());
    assert!(!temp.path().join("bulk-backups").exists());
    assert!(
        fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
            .expect("toc")
            .contains("1.0.0")
    );
}

#[test]
fn update_addons_from_index_skips_ignored_preflight_match_without_provider_prepare() {
    #[derive(Default)]
    struct FakeProvider {
        materialize_ref_calls: Cell<usize>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            self.materialize_ref_calls
                .set(self.materialize_ref_calls.get() + 1);
            Err(AppError::Validation(format!(
                "ignored package should not be prepared: {}",
                request.source.display_name()
            )))
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked curseforge addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: Some(true),
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: None,
    })
    .expect("set ignored policy");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("ignored index update should not prepare provider source");

    assert_eq!(provider.materialize_ref_calls.get(), 0);
    assert!(result.selected_packages.is_empty());
    assert_eq!(
        result.update.ignored_packages,
        vec!["curseforge-42".to_string()]
    );
    assert!(result.update.backup_path.is_none());
}

#[test]
fn update_addons_from_index_installs_missing_required_dependencies_when_policy_enabled() {
    #[derive(Default)]
    struct FakeProvider {
        dependency_requests: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    request.stage_root.join("curse-addon-update.zip")
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    request.stage_root.join("sharedmedia-addon.zip")
                }
                source => {
                    return Err(AppError::Validation(format!(
                        "unexpected source during addon-index dependency test: {}",
                        source.display_name()
                    )));
                }
            };

            let entries = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => vec![(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => vec![(
                    "SharedMedia/SharedMedia.toc",
                    "## Interface: 120000\n## Version: 1.0.0\n",
                )],
                _ => unreachable!(),
            };
            create_addon_archive(&archive_path, &entries);

            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn dependency_resolution_capability(
            &self,
            source: &AddonSourceRef,
        ) -> AddonDependencyResolutionCapability {
            match source {
                AddonSourceRef::CurseForgeMod { .. } => {
                    AddonDependencyResolutionCapability::missing_required_only()
                }
                _ => AddonDependencyResolutionCapability::Unsupported,
            }
        }

        fn resolve_addon_dependencies(
            &self,
            request: ResolveAddonDependenciesRequest<'_>,
        ) -> AppResult<ResolvedAddonDependencies> {
            self.dependency_requests
                .borrow_mut()
                .push(request.source.clone());
            match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(vec![
                        AddonSourceRef::CurseForgeMod {
                            mod_id: 99,
                            file_id: None,
                        },
                    ]))
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(Vec::new()))
                }
                source => Err(AppError::Validation(format!(
                    "unexpected source during addon-index dependency resolution test: {}",
                    source.display_name()
                ))),
            }
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked curseforge addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index with dependency installation");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.installed_dependency_packages.len(), 1);
    assert_eq!(
        result.update.installed_dependency_packages[0].package_id,
        "curseforge-99"
    );
    assert_eq!(
        provider.dependency_requests.borrow().as_slice(),
        &[
            AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: None,
            },
            AddonSourceRef::CurseForgeMod {
                mod_id: 99,
                file_id: None,
            },
        ]
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 2);
    assert!(
        inventory
            .tracked_packages
            .iter()
            .any(|package| package.package_id == "curseforge-42")
    );
    assert!(
        inventory
            .tracked_packages
            .iter()
            .any(|package| package.package_id == "curseforge-99")
    );
}

#[test]
fn update_addons_from_index_rolls_back_when_dependency_install_fails_after_primary_update() {
    #[derive(Default)]
    struct FakeProvider {
        dependency_requests: RefCell<Vec<AddonSourceRef>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    request.stage_root.join("curse-addon-update.zip")
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    request.stage_root.join("sharedmedia-addon.zip")
                }
                source => {
                    return Err(AppError::Validation(format!(
                        "unexpected source during addon-index dependency rollback test: {}",
                        source.display_name()
                    )));
                }
            };

            let entries = match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => vec![(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => vec![(
                    "SharedMedia/SharedMedia.toc",
                    "## Interface: 120000\n## Version: 1.0.0\n",
                )],
                _ => unreachable!(),
            };
            create_addon_archive(&archive_path, &entries);

            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn dependency_resolution_capability(
            &self,
            source: &AddonSourceRef,
        ) -> AddonDependencyResolutionCapability {
            match source {
                AddonSourceRef::CurseForgeMod { .. } => {
                    AddonDependencyResolutionCapability::missing_required_only()
                }
                _ => AddonDependencyResolutionCapability::Unsupported,
            }
        }

        fn resolve_addon_dependencies(
            &self,
            request: ResolveAddonDependenciesRequest<'_>,
        ) -> AppResult<ResolvedAddonDependencies> {
            self.dependency_requests
                .borrow_mut()
                .push(request.source.clone());
            match request.source {
                AddonSourceRef::CurseForgeMod { mod_id: 42, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(vec![
                        AddonSourceRef::CurseForgeMod {
                            mod_id: 99,
                            file_id: None,
                        },
                    ]))
                }
                AddonSourceRef::CurseForgeMod { mod_id: 99, .. } => {
                    Ok(ResolvedAddonDependencies::missing_required_only(Vec::new()))
                }
                source => Err(AppError::Validation(format!(
                    "unexpected source during addon-index dependency rollback resolution test: {}",
                    source.display_name()
                ))),
            }
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked curseforge addon");

    let local_dependency_dir = installation.addon_dir.join("SharedMedia");
    fs::create_dir_all(&local_dependency_dir).expect("create local dependency conflict");
    fs::write(
        local_dependency_dir.join("SharedMedia.toc"),
        "## Interface: 110000\n## Version: local\n",
    )
    .expect("write local dependency conflict");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let error = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("dependency install conflict should roll back");

    let message = error.to_string();
    assert!(message.contains("rollback restored"));
    assert!(message.contains("addon directory already exists"));
    assert!(
        fs::read_to_string(
            installation
                .addon_dir
                .join("WeakAuras")
                .join("WeakAuras.toc")
        )
        .expect("weakauras toc after rollback")
        .contains("1.0.0")
    );
    assert!(
        fs::read_to_string(local_dependency_dir.join("SharedMedia.toc"))
            .expect("local dependency after rollback")
            .contains("local")
    );

    let inventory =
        list_addons(&installation, &addon_state_paths(&installation)).expect("inventory");
    assert_eq!(inventory.tracked_packages.len(), 1);
    assert_eq!(inventory.tracked_packages[0].package_id, "curseforge-42");
}

#[test]
fn update_addons_from_index_keeps_curated_source_authority_over_pin_and_release_policy() {
    #[derive(Default)]
    struct FakeProvider {
        update_requests: RefCell<Vec<(AddonSourceRef, AddonSourceResolutionPolicy)>>,
    }

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("curse-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::CurseForgeMod {
                    mod_id: 42,
                    file_id: None,
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            self.update_requests
                .borrow_mut()
                .push((request.source.clone(), request.context.resolution_policy()));
            let archive_path = request.stage_root.join("curse-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "WeakAuras/WeakAuras.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider::default();
    let index_path = write_index_package(
        temp.path(),
        "weakauras",
        "WeakAuras",
        "2.0.0",
        r#"{ kind = "curseforge_mod", mod_id = 42 }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "curseforge:42".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked curseforge addon");

    provider.update_requests.borrow_mut().clear();

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "curseforge-42".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: Some(777),
        release_channel: Some(AddonReleaseChannel::Alpha),
        allow_prerelease: Some(true),
        install_dependencies: None,
    })
    .expect("set source override policy");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index with curated source authority");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(
        result.update.updated_packages[0].source,
        AddonSourceRef::CurseForgeMod {
            mod_id: 42,
            file_id: None,
        }
    );
    assert_eq!(
        provider.update_requests.borrow().as_slice(),
        &[(
            AddonSourceRef::CurseForgeMod {
                mod_id: 42,
                file_id: None,
            },
            AddonSourceResolutionPolicy::default(),
        )]
    );
}

#[test]
fn update_addons_from_index_matches_tracked_package_by_source_family_identity_when_github_asset_changes()
 {
    #[derive(Default)]
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider;
    let index_path = write_index_package(
        temp.path(),
        "curated-plater",
        "Curated Plater",
        "2.0.0",
        r#"{ kind = "github_release", owner = "owner", repo = "repo", asset_name = "release.zip" }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index by source family identity");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.selected_packages[0].id, "curated-plater");
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.updated_packages[0].package_id, "plater");
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_from_index_matches_tracked_package_by_display_name_when_source_family_changes() {
    #[derive(Default)]
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "legacy-owner".to_string(),
                    repo: "legacy-repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Title: Plater\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider;
    let index_path = write_index_package(
        temp.path(),
        "curated-plater",
        "Plater",
        "2.0.0",
        r#"{ kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index by display name continuity");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.selected_packages[0].id, "curated-plater");
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.updated_packages[0].package_id, "plater");
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_from_index_matches_tracked_package_by_curated_package_hint_when_source_family_changes()
 {
    #[derive(Default)]
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "legacy-owner".to_string(),
                    repo: "legacy-repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider;
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater-v2"
name = "Curated Plater Package"
version = "2.0.0"
match_package_ids = ["plater"]
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

    let result = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            index_path,
            name: Some("curated-plater-v2".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("update from index by curated package hint");

    assert_eq!(result.selected_packages.len(), 1);
    assert_eq!(result.selected_packages[0].id, "curated-plater-v2");
    assert_eq!(
        result.selected_packages[0].match_package_ids,
        vec!["plater".to_string()]
    );
    assert_eq!(result.update.updated_packages.len(), 1);
    assert_eq!(result.update.updated_packages[0].package_id, "plater");
    assert!(
        fs::read_to_string(installation.addon_dir.join("Plater").join("Plater.toc"))
            .expect("toc")
            .contains("2.0.0")
    );
}

#[test]
fn update_addons_from_index_rejects_dependency_installation_for_unsupported_sources() {
    #[derive(Default)]
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider;
    let index_path = write_index_package(
        temp.path(),
        "curated-plater",
        "Curated Plater",
        "2.0.0",
        r#"{ kind = "github_release", owner = "owner", repo = "repo", asset_name = "release.zip" }"#,
    );
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:owner/repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let error = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: Some("curated-plater".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("unsupported dependency installation should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
}

#[test]
fn update_addons_from_index_explains_deferred_dependency_policy_failure_when_preflight_cannot_match()
 {
    #[derive(Default)]
    struct FakeProvider;

    impl AddonProvider for FakeProvider {
        fn materialize_source_input(
            &self,
            request: MaterializeSourceInputRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 110000\n## Version: 1.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: AddonSourceRef::GitHubRelease {
                    owner: "legacy-owner".to_string(),
                    repo: "legacy-repo".to_string(),
                    tag: None,
                    asset_name: Some("plater.zip".to_string()),
                },
                archive_path,
            })
        }

        fn materialize_source_ref(
            &self,
            request: MaterializeSourceRefRequest<'_>,
        ) -> AppResult<MaterializedAddonSource> {
            let archive_path = request.stage_root.join("github-addon-update.zip");
            create_addon_archive(
                &archive_path,
                &[(
                    "Plater/Plater.toc",
                    "## Interface: 120000\n## Version: 2.0.0\n",
                )],
            );
            Ok(MaterializedAddonSource {
                source_ref: request.source.clone(),
                archive_path,
            })
        }

        fn search_addons(
            &self,
            _request: ProviderAddonSearchRequest<'_>,
        ) -> AppResult<Vec<AddonSearchResult>> {
            Ok(Vec::new())
        }
    }

    let temp = tempdir().expect("temp dir");
    let installation = create_fixture_installation(temp.path());
    let provider = FakeProvider;
    let index_path = temp.path().join("index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater-v3"
name = "Curated Plater Package"
version = "2.0.0"
source = { kind = "github_release", owner = "new-owner", repo = "new-repo", asset_name = "release.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");
    let mut progress = NoopProgressSink;

    install_addon_task_with_provider(
        &provider,
        InstallAddonRequest {
            state_paths: addon_state_paths(&installation.clone()),
            installation: installation.clone(),
            source: "github:legacy-owner/legacy-repo#plater.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        },
        &NeverCancel,
        &mut progress,
    )
    .expect("install tracked github addon");

    set_addon_policy(SetAddonPolicyRequest {
        state_paths: addon_state_paths(&installation.clone()),
        installation: installation.clone(),
        package: "plater".to_string(),
        ignored: None,
        pinned_version: None,
        pinned_file_id: None,
        release_channel: None,
        allow_prerelease: None,
        install_dependencies: Some(true),
    })
    .expect("enable dependency installation");

    let error = update_addons_from_index_task_with_provider(
        &provider,
        AddonIndexUpdateRequest {
            state_paths: addon_state_paths(&installation),
            installation,
            index_path,
            name: Some("curated-plater-v3".to_string()),
            dry_run: false,
            backup_output_path: Some(temp.path().join("bulk-backups")),
        },
        &NeverCancel,
        &mut progress,
    )
    .expect_err("deferred dependency-policy validation should fail with guidance");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("not supported"));
    assert!(
        error
            .to_string()
            .contains("app preflight could not determine")
    );
    assert!(error.to_string().contains("match_package_ids"));
    assert!(error.to_string().contains("addon_directories"));
}

fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
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

fn write_index(root: &Path, archive_path: &Path) -> std::path::PathBuf {
    write_index_package(
        root,
        "details",
        "Details",
        "1.0.0",
        &format!(
            r#"{{ kind = "local_archive", path = "{}" }}"#,
            archive_path.display().to_string().replace('\\', "\\\\")
        ),
    )
}

fn write_index_package(
    root: &Path,
    package_id: &str,
    package_name: &str,
    version: &str,
    source_toml: &str,
) -> std::path::PathBuf {
    let index_path = root.join("index.toml");
    fs::write(
        &index_path,
        format!(
            r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "{package_id}"
name = "{package_name}"
version = "{version}"
source = {source_toml}
supported_flavors = ["retail"]
"#
        ),
    )
    .expect("index");
    index_path
}

fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
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

fn normalized_archive_path(path: &Path) -> std::path::PathBuf {
    canonicalize_local_archive_path(path).expect("normalized archive path")
}

fn assert_addon_index_task_progress(
    events: &[TaskProgressEvent],
    task: TaskKind,
    executing_detail: &str,
) {
    let phases = events
        .iter()
        .map(|event| (event.task, event.phase))
        .collect::<Vec<_>>();

    assert_eq!(phases.first(), Some(&(task, TaskPhase::Preparing)));
    assert_eq!(phases.last(), Some(&(task, TaskPhase::Completed)));
    assert!(phases.contains(&(task, TaskPhase::BackingUp)));
    assert!(phases.contains(&(task, TaskPhase::Executing)));
    assert!(events.iter().any(|event| {
        event.task == task
            && event.phase == TaskPhase::Executing
            && event.message.contains(executing_detail)
    }));
}
