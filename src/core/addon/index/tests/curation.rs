use std::fs;

use tempfile::tempdir;

use super::super::{
    AddonIndexPackageSuggestionStatus, AddonIndexScaffoldRequest, AddonIndexSuggestionRequest,
    AddonIndexTrackedMatchStrategy, inspect_addon_index, scaffold_addon_index,
    suggest_addon_index_hints,
};
use super::{addon_state_paths, create_addon_archive, create_fixture_installation};
use crate::core::addon::{AddonPackageMetadata, InstallAddonRequest, install_addon, list_addons};
use crate::core::error::AppError;

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
        AddonIndexPackageSuggestionStatus::Suggested
    ));
    assert_eq!(
        curated.matched_tracked_package_id.as_deref(),
        Some("plater")
    );
    assert!(matches!(
        curated.match_strategy,
        Some(AddonIndexTrackedMatchStrategy::SourceIdentity)
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
        AddonIndexPackageSuggestionStatus::NoLocalMatch
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
        AddonIndexPackageSuggestionStatus::Complete
    ));
    assert_eq!(package.match_package_ids_to_add, Vec::<String>::new());
    assert_eq!(package.addon_directories_to_add, Vec::<String>::new());
    assert!(matches!(
        package.match_strategy,
        Some(AddonIndexTrackedMatchStrategy::ExactPackageId)
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
        AddonIndexPackageSuggestionStatus::AmbiguousLocalMatch
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
        metadata: Some(AddonPackageMetadata {
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
