use super::*;

#[test]
fn analyze_external_package_serializes_summary_groups_for_machine_consumers() {
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

    let wtf_scopes = summary
        .get("wtf_scopes")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("wtf_scopes array");

    assert_eq!(
        wtf_scopes,
        vec![
            serde_json::json!({
                "scope": "root_saved_variables",
                "risk": "high",
                "count": 1
            }),
            serde_json::json!({
                "scope": "character_saved_variables",
                "risk": "medium",
                "count": 1
            }),
            serde_json::json!({
                "scope": "cache_like",
                "risk": "low",
                "count": 1
            }),
        ]
    );

    let sensitive_wtf_files = summary
        .get("sensitive_wtf_files")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .expect("sensitive_wtf_files array");

    assert_eq!(
        sensitive_wtf_files,
        vec![
            serde_json::json!({
                "kind": "saved_variables",
                "severity": "review_required",
                "count": 2
            }),
            serde_json::json!({
                "kind": "game_config",
                "severity": "advisory",
                "count": 1
            }),
        ]
    );

    let source_identities = summary
        .get("source_identities")
        .cloned()
        .expect("source_identities object");

    assert_eq!(
        source_identities,
        serde_json::json!({
            "source_accounts": ["ACC1"],
            "source_characters": [
                {
                    "source_account": "ACC1",
                    "source_server": "Illidan",
                    "source_character": "Targetone"
                }
            ],
            "entries_with_source_account": 2,
            "entries_with_source_character": 1
        })
    );

    let public_sharing = summary
        .get("public_sharing")
        .cloned()
        .expect("public_sharing object");

    assert_eq!(
        public_sharing,
        serde_json::json!({
            "status": "review_required",
            "public_ready": false,
            "review_required_count": 6,
            "advisory_count": 2,
            "reasons": [
                {
                    "severity": "review_required",
                    "code": "normalization_warnings",
                    "count": 1,
                    "message": "package normalization produced warnings; review ignored or unsupported files before public sharing"
                },
                {
                    "severity": "review_required",
                    "code": "high_risk_wtf_scope",
                    "count": 1,
                    "message": "package contains account-wide SavedVariables or other high-risk WTF data"
                },
                {
                    "severity": "review_required",
                    "code": "medium_risk_wtf_scope",
                    "count": 1,
                    "message": "package contains global, account-root, character SavedVariables, or character-state WTF data"
                },
                {
                    "severity": "advisory",
                    "code": "low_risk_wtf_scope",
                    "count": 1,
                    "message": "package contains cache-like WTF data; it is low risk but still worth reviewing"
                },
                {
                    "severity": "review_required",
                    "code": "sensitive_wtf_file",
                    "count": 2,
                    "message": "package contains WTF files known to carry private addon state, chat history, macros, or SavedVariables"
                },
                {
                    "severity": "advisory",
                    "code": "advisory_wtf_file",
                    "count": 1,
                    "message": "package contains WTF state files that are usually shareable only after review"
                },
                {
                    "severity": "review_required",
                    "code": "source_account_identity",
                    "count": 2,
                    "message": "package paths expose source account identity"
                },
                {
                    "severity": "review_required",
                    "code": "source_character_identity",
                    "count": 1,
                    "message": "package paths expose source character and realm identity"
                }
            ]
        })
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
fn public_sharing_reason_code_serialization_matches_display_codes() {
    let codes = [
        ExternalPackagePublicSharingReasonCode::NormalizationWarnings,
        ExternalPackagePublicSharingReasonCode::HighRiskWtfScope,
        ExternalPackagePublicSharingReasonCode::MediumRiskWtfScope,
        ExternalPackagePublicSharingReasonCode::LowRiskWtfScope,
        ExternalPackagePublicSharingReasonCode::UnknownRiskWtfScope,
        ExternalPackagePublicSharingReasonCode::SensitiveWtfFile,
        ExternalPackagePublicSharingReasonCode::AdvisoryWtfFile,
        ExternalPackagePublicSharingReasonCode::SourceAccountIdentity,
        ExternalPackagePublicSharingReasonCode::SourceCharacterIdentity,
    ];

    for code in codes {
        let serialized = serde_json::to_value(code)
            .expect("serialize public sharing reason code")
            .as_str()
            .map(str::to_string)
            .expect("public sharing reason code string");
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
