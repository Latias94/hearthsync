use super::*;

#[test]
fn addon_index_service_inspects_index_file() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
match_package_ids = ["legacy-weakauras"]
source = { kind = "local_archive", path = "WeakAuras.zip" }
"#,
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let inspection = service
        .inspect(InspectAddonIndexRequest { index_path })
        .expect("inspect addon index");

    assert_eq!(inspection.package_count, 1);
    assert_eq!(inspection.name, "Fixture Index");
    assert_eq!(inspection.packages[0].id, "weakauras");
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
        1
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
        0
    );
    assert_eq!(
        inspection
            .identity_hint_coverage
            .package_count_without_match_package_ids,
        0
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
        0
    );
    assert_eq!(inspection.warning_count, 1);
    assert_eq!(inspection.blocking_warning_count, 0);
    assert_eq!(inspection.advisory_warning_count, 1);
    assert_eq!(inspection.warnings.len(), 1);
    assert!(matches!(
        inspection.warnings[0].code,
        AddonIndexInspectionWarningCodeResult::MissingAddonDirectories
    ));
    assert!(matches!(
        inspection.warnings[0].severity,
        AddonIndexInspectionWarningSeverityResult::Advisory
    ));
    assert_eq!(
        inspection.packages[0].match_package_ids,
        vec!["legacy-weakauras".to_string()]
    );
    assert_eq!(
        inspection.packages[0]
            .source
            .dependency_resolution_capability,
        AddonDependencyResolutionCapabilityValue::Unsupported
    );
}

#[test]
fn addon_index_service_inspects_relative_index_against_runtime_base() {
    let temp = tempdir().expect("temp dir");
    let index_dir = temp.path().join("indexes");
    fs::create_dir_all(&index_dir).expect("index dir");
    fs::write(
        index_dir.join("addon-index.toml"),
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
addon_directories = ["WeakAuras"]
source = { kind = "local_archive", path = "WeakAuras.zip" }
"#,
    )
    .expect("write index");

    let service = AddonIndexService::with_runtime(
        AppRuntime::builder()
            .with_relative_path_base(Some(index_dir.clone()))
            .build()
            .expect("runtime"),
    );
    let inspection = service
        .inspect(InspectAddonIndexRequest {
            index_path: PathBuf::from("addon-index.toml"),
        })
        .expect("inspect relative addon index");

    assert_eq!(inspection.index_path, index_dir.join("addon-index.toml"));
    assert_eq!(inspection.package_count, 1);
}

#[test]
fn addon_index_service_rejects_relative_index_without_runtime_base() {
    let service = AddonIndexService::new();
    let error = service
        .inspect(InspectAddonIndexRequest {
            index_path: PathBuf::from("addon-index.toml"),
        })
        .expect_err("relative addon index without base should fail");

    assert!(matches!(error, AppError::Validation(_)));
    assert!(error.to_string().contains("relative path base"));
}

#[test]
fn addon_index_service_validate_reports_warning_summary() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "curated-plater"
name = "Curated Plater"
version = "1.0.0"
source = { kind = "http_archive", url = "https://example.invalid/plater.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .validate(InspectAddonIndexRequest { index_path })
        .expect("validate addon index");

    assert!(!result.valid);
    assert_eq!(result.warning_count, 1);
    assert_eq!(result.blocking_warning_count, 1);
    assert_eq!(result.advisory_warning_count, 0);
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        result.warnings[0].code,
        AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints
    ));
    assert!(matches!(
        result.warnings[0].severity,
        AddonIndexInspectionWarningSeverityResult::Blocking
    ));
    assert_eq!(result.warnings[0].package_id, "curated-plater");
}

#[test]
fn addon_index_service_validate_keeps_advisory_identity_gaps_non_blocking() {
    let temp = tempdir().expect("temp dir");
    let index_path = temp.path().join("addon-index.toml");
    fs::write(
        &index_path,
        r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "weakauras"
name = "WeakAuras"
version = "1.0.0"
match_package_ids = ["legacy-weakauras"]
source = { kind = "http_archive", url = "https://example.invalid/weakauras.zip" }
supported_flavors = ["retail"]
"#,
    )
    .expect("write index");

    let service = AddonIndexService::new();
    let result = service
        .validate(InspectAddonIndexRequest { index_path })
        .expect("validate addon index");

    assert!(result.valid);
    assert_eq!(result.warning_count, 1);
    assert_eq!(result.blocking_warning_count, 0);
    assert_eq!(result.advisory_warning_count, 1);
    assert_eq!(result.warnings.len(), 1);
    assert!(matches!(
        result.warnings[0].code,
        AddonIndexInspectionWarningCodeResult::MissingAddonDirectories
    ));
    assert!(matches!(
        result.warnings[0].severity,
        AddonIndexInspectionWarningSeverityResult::Advisory
    ));
    assert_eq!(result.warnings[0].package_id, "weakauras");
}
