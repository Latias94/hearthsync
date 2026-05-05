use super::format_external_package_warnings;
use crate::core::app::{
    ExternalPackagePublicSharingStatusValue, ExternalPackagePublicSharingSummaryResult,
    ExternalPackageSourceIdentityResult, ExternalPackageSummaryResult,
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
    ExternalPackageWarningGroupResult, ExternalPackageWarningResult,
    ExternalPackageWtfScopeSummaryResult, WtfScopeRiskValue, WtfScopeValue,
};

#[test]
fn format_external_package_warnings_renders_groups_and_details() {
    let warnings = vec![
        ExternalPackageWarningResult {
            category: ExternalPackageWarningCategoryValue::Addon,
            code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
            source_path: "AuthorUI/Interface/AddOns/BrokenAddon/README.txt".to_string(),
            message: "ignored addon entry".to_string(),
        },
        ExternalPackageWarningResult {
            category: ExternalPackageWarningCategoryValue::Wtf,
            code: ExternalPackageWarningCodeValue::WtfSavedVariablesPathWithoutFile,
            source_path: "AuthorUI/WTF/Account/ACCOUNT/SavedVariables".to_string(),
            message: "unsupported wtf entry".to_string(),
        },
    ];
    let summary = ExternalPackageSummaryResult {
        warning_count: 2,
        addon_warning_count: 1,
        wtf_warning_count: 1,
        warning_groups: vec![
            ExternalPackageWarningGroupResult {
                category: ExternalPackageWarningCategoryValue::Addon,
                code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
                count: 1,
            },
            ExternalPackageWarningGroupResult {
                category: ExternalPackageWarningCategoryValue::Wtf,
                code: ExternalPackageWarningCodeValue::WtfSavedVariablesPathWithoutFile,
                count: 1,
            },
        ],
        wtf_scopes: vec![ExternalPackageWtfScopeSummaryResult {
            scope: WtfScopeValue::AccountSavedVariables,
            risk: WtfScopeRiskValue::High,
            count: 1,
        }],
        source_identities: ExternalPackageSourceIdentityResult {
            source_accounts: Vec::new(),
            source_characters: Vec::new(),
            entries_with_source_account: 0,
            entries_with_source_character: 0,
        },
        public_sharing: ExternalPackagePublicSharingSummaryResult {
            status: ExternalPackagePublicSharingStatusValue::Ready,
            public_ready: true,
            review_required_count: 0,
            advisory_count: 0,
            reasons: Vec::new(),
        },
        total_files: 0,
        normalized_files: 0,
        ignored_files: 0,
        addons: 0,
        wtf_common: 0,
        wtf_characters: 0,
        fonts: 0,
        interface_assets: 0,
    };

    let rendered = format_external_package_warnings(&warnings, &summary);

    assert!(rendered.contains("2 (addon: 1, wtf: 1; groups: ["));
    assert!(rendered.contains("addon/addon_root_not_detected=1"));
    assert!(rendered.contains("wtf/wtf_savedvariables_path_without_file=1"));
    assert!(rendered.contains(
        "addon/addon_root_not_detected: AuthorUI/Interface/AddOns/BrokenAddon/README.txt"
    ));
    assert!(rendered.contains(
        "wtf/wtf_savedvariables_path_without_file: AuthorUI/WTF/Account/ACCOUNT/SavedVariables"
    ));
}

#[test]
fn format_external_package_warnings_returns_none_for_empty_warnings() {
    let warnings: [ExternalPackageWarningResult; 0] = [];
    let rendered = format_external_package_warnings(
        &warnings,
        &ExternalPackageSummaryResult {
            total_files: 0,
            normalized_files: 0,
            ignored_files: 0,
            addons: 0,
            wtf_common: 0,
            wtf_characters: 0,
            fonts: 0,
            interface_assets: 0,
            warning_count: 0,
            addon_warning_count: 0,
            wtf_warning_count: 0,
            warning_groups: Vec::new(),
            wtf_scopes: Vec::new(),
            source_identities: ExternalPackageSourceIdentityResult {
                source_accounts: Vec::new(),
                source_characters: Vec::new(),
                entries_with_source_account: 0,
                entries_with_source_character: 0,
            },
            public_sharing: ExternalPackagePublicSharingSummaryResult {
                status: ExternalPackagePublicSharingStatusValue::Ready,
                public_ready: true,
                review_required_count: 0,
                advisory_count: 0,
                reasons: Vec::new(),
            },
        },
    );

    assert_eq!(rendered, "none");
}
