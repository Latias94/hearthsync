use super::*;

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
