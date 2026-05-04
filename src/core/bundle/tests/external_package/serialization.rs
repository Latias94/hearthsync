use super::*;

#[test]
fn analyze_external_package_serializes_warning_groups_for_machine_consumers() {
    let analysis = analyze_external_package(AnalyzeExternalPackageRequest::new(
        external_package_dirty_fixture_root(),
    ))
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

#[test]
fn external_package_layout_serialization_uses_public_layout_names() {
    let layouts = [
        (ExternalPackageLayout::Auto, "auto"),
        (ExternalPackageLayout::Generic, "generic"),
        (ExternalPackageLayout::NewBeeBoxAddon, "newbeebox_addon"),
        (ExternalPackageLayout::NewBeeBoxFont, "newbeebox_font"),
        (
            ExternalPackageLayout::NewBeeBoxMaterial,
            "newbeebox_material",
        ),
        (
            ExternalPackageLayout::NewBeeBoxWtfAccount,
            "newbeebox_wtf_account",
        ),
        (
            ExternalPackageLayout::NewBeeBoxWtfCharacter,
            "newbeebox_wtf_character",
        ),
    ];

    for (layout, expected) in layouts {
        let serialized = serde_json::to_value(layout)
            .expect("serialize layout")
            .as_str()
            .map(str::to_string)
            .expect("layout string");
        assert_eq!(serialized, expected, "unexpected layout serialization");
    }
}
