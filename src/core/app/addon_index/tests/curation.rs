use super::*;

#[test]
fn addon_index_service_suggests_exact_identity_hints_from_local_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");

    let index_path = temp.path().join("addon-index.toml");
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
"#,
            archive_path.display().to_string().replace('\\', "\\\\"),
        ),
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .suggest(SuggestAddonIndexRequest {
            installation,
            index_path,
            name: None,
        })
        .expect("suggest addon index hints");

    assert_eq!(result.index_name, "Fixture Index");
    assert_eq!(result.considered_package_count, 1);
    assert_eq!(result.suggested_package_count, 1);
    assert_eq!(result.complete_package_count, 0);
    assert_eq!(result.no_match_package_count, 0);
    let package = &result.packages[0];
    assert_eq!(package.package_id, "curated-plater");
    assert!(matches!(
        package.status,
        AddonIndexPackageSuggestionStatusResult::Suggested
    ));
    assert_eq!(
        package.matched_tracked_package_id.as_deref(),
        Some("plater")
    );
    assert!(matches!(
        package.match_strategy,
        Some(AddonIndexTrackedMatchStrategyResult::SourceIdentity)
    ));
    assert_eq!(package.match_package_ids_to_add, vec!["plater".to_string()]);
    assert_eq!(package.addon_directories_to_add, vec!["Plater".to_string()]);
}

#[test]
fn addon_index_service_scaffolds_index_from_tracked_registry() {
    let temp = tempdir().expect("temp dir");
    let installation = create_empty_installation(temp.path());
    let archive_path = temp.path().join("Plater.zip");
    create_addon_archive(
        &archive_path,
        &[(
            "Plater/Plater.toc",
            "## Interface: 110000\n## Title: Plater\n## Version: 1.0.0\n",
        )],
    );

    AddonService::new()
        .install(InstallAddonAppRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install tracked addon");

    let index_path = temp.path().join("addon-index.toml");
    let service = AddonIndexService::new();
    let result: AddonIndexScaffoldResult = service
        .scaffold(ScaffoldAddonIndexRequest {
            installation,
            index_path: index_path.clone(),
            index_name: "Guild UI".to_string(),
            description: Some("Scaffolded".to_string()),
            name: Some("plater".to_string()),
            overwrite: false,
        })
        .expect("scaffold addon index");

    assert_eq!(result.index_path, index_path);
    assert_eq!(result.index_name, "Guild UI");
    assert_eq!(result.package_count, 1);
    assert_eq!(result.package_ids, vec!["plater".to_string()]);
    assert_eq!(result.inferred_name_package_count, 1);
    assert_eq!(result.inferred_version_package_count, 1);
    assert_eq!(result.placeholder_version_package_count, 0);
    assert!(index_path.exists());
}
