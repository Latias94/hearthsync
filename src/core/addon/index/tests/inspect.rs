use std::fs;

use tempfile::tempdir;

use super::super::{
    AddonIndexInspectionWarningCode, AddonIndexInspectionWarningSeverity, inspect_addon_index,
};
use super::{write_index, write_index_package};
use crate::core::addon::AddonSourceRef;
use crate::core::error::AppError;

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
        AddonIndexInspectionWarningCode::MissingExactIdentityHints
    ));
    assert!(matches!(
        inspection.warnings[0].severity,
        AddonIndexInspectionWarningSeverity::Blocking
    ));
    assert_eq!(inspection.warnings[0].package_id, "details");
    assert!(
        inspection.warnings[0]
            .message
            .contains("does not declare exact identity hints")
    );
}

#[test]
fn inspect_addon_index_reads_wago_sources() {
    let temp = tempdir().expect("temp dir");
    let index_path = write_index_package(
        temp.path(),
        "details",
        "Details",
        "1.0.0",
        r#"{ kind = "wago_addon", project_id = "qv63A7Gb", release_id = "vdx1042w" }"#,
    );

    let inspection = inspect_addon_index(&index_path).expect("inspect wago index");

    assert_eq!(
        inspection.index.packages[0].source,
        AddonSourceRef::WagoAddon {
            project_id: "qv63A7Gb".to_string(),
            release_id: Some("vdx1042w".to_string()),
        }
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
fn inspect_addon_index_rejects_invalid_source_refs() {
    for (source_toml, expected_message) in [
        (
            r#"{ kind = "local_archive", path = "" }"#,
            "local archive source path",
        ),
        (
            r#"{ kind = "http_archive", url = "" }"#,
            "HTTP archive source URL",
        ),
        (
            r#"{ kind = "http_archive", url = "ftp://example.invalid/details.zip" }"#,
            "HTTP archive source URL",
        ),
        (
            r#"{ kind = "curseforge_mod", mod_id = 0 }"#,
            "CurseForge mod id",
        ),
        (
            r#"{ kind = "curseforge_mod", mod_id = 12345, file_id = 0 }"#,
            "CurseForge file id",
        ),
        (
            r#"{ kind = "github_release", owner = "", repo = "details" }"#,
            "GitHub owner",
        ),
        (
            r#"{ kind = "github_release", owner = "owner", repo = " " }"#,
            "GitHub repo",
        ),
        (
            r#"{ kind = "github_release", owner = "owner", repo = "details", tag = "" }"#,
            "GitHub tag",
        ),
        (
            r#"{ kind = "github_release", owner = "owner", repo = "details", asset_name = "" }"#,
            "GitHub asset name",
        ),
        (
            r#"{ kind = "wago_addon", project_id = "" }"#,
            "Wago project id",
        ),
        (
            r#"{ kind = "wago_addon", project_id = "qv63A7Gb", release_id = "bad/id" }"#,
            "Wago release id",
        ),
    ] {
        let temp = tempdir().expect("temp dir");
        let index_path =
            write_index_package(temp.path(), "details", "Details", "1.0.0", source_toml);

        let error = inspect_addon_index(&index_path).expect_err("invalid source should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(error.to_string().contains("invalid source"));
        assert!(error.to_string().contains(expected_message));
        assert!(error.to_string().contains("for package `details`"));
    }
}

#[test]
fn inspect_addon_index_rejects_blank_package_metadata_fields() {
    for (field, field_toml) in [
        ("source_url", r#"source_url = " ""#),
        ("website_url", r#"website_url = """#),
        ("sha256", r#"sha256 = " ""#),
    ] {
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
source = {{ kind = "http_archive", url = "https://example.invalid/details.zip" }}
supported_flavors = ["retail"]
{field_toml}
"#
            ),
        )
        .expect("write index");

        let error = inspect_addon_index(&index_path).expect_err("blank metadata should fail");

        assert!(matches!(error, AppError::Validation(_)));
        assert!(error.to_string().contains(&format!("`{field}`")));
        assert!(error.to_string().contains("must not be blank"));
        assert!(error.to_string().contains("for package `details`"));
    }
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
            AddonIndexInspectionWarningCode::MissingMatchPackageIds
        ) && matches!(
            warning.severity,
            AddonIndexInspectionWarningSeverity::Advisory
        ) && warning.package_id == "directory-bridge"
    }));
    assert!(inspection.warnings.iter().any(|warning| {
        matches!(
            warning.code,
            AddonIndexInspectionWarningCode::MissingAddonDirectories
        ) && matches!(
            warning.severity,
            AddonIndexInspectionWarningSeverity::Advisory
        ) && warning.package_id == "legacy-bridge"
    }));
    assert!(inspection.warnings.iter().any(|warning| {
        matches!(
            warning.code,
            AddonIndexInspectionWarningCode::MissingExactIdentityHints
        ) && matches!(
            warning.severity,
            AddonIndexInspectionWarningSeverity::Blocking
        ) && warning.package_id == "no-bridge"
    }));
}
