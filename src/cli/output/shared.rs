use std::path::Path;

use crate::core::app::{
    BundleCharacterResourceResult, CharacterMappingResult, ExternalPackageSummaryResult,
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
    ExternalPackageWarningResult, LocalWowAccountResult,
};

pub(super) fn format_bundle_characters(resources: &[BundleCharacterResourceResult]) -> String {
    let characters = resources
        .iter()
        .map(|character| {
            format!(
                "{}/{}/{}",
                character
                    .source_account
                    .as_deref()
                    .unwrap_or("<unknown-account>"),
                character.source_server,
                character.source_character
            )
        })
        .collect::<Vec<_>>();

    if characters.is_empty() {
        "none".to_string()
    } else {
        characters.join(", ")
    }
}

pub(super) fn format_discovered_accounts(accounts: &[LocalWowAccountResult]) -> String {
    if accounts.is_empty() {
        "none".to_string()
    } else {
        accounts
            .iter()
            .map(|account| {
                format!(
                    "{}({} chars)",
                    account.account_name,
                    account.characters.len()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(super) fn format_string_list_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

pub(super) fn format_optional_path_or_none(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| "none".to_string())
}

pub(super) fn format_selected_accounts(accounts: &[String]) -> String {
    format_string_list_or_none(accounts)
}

pub(super) fn format_character_mapping_summary(mappings: &[CharacterMappingResult]) -> String {
    if mappings.is_empty() {
        "none".to_string()
    } else {
        format_character_mappings(mappings)
    }
}

pub(super) fn format_character_mappings(mappings: &[CharacterMappingResult]) -> String {
    mappings
        .iter()
        .map(|mapping| {
            format!(
                "{}/{}/{} -> {}/{}/{}",
                mapping
                    .source_account
                    .as_deref()
                    .unwrap_or("<unknown-account>"),
                mapping.source_server,
                mapping.source_character,
                mapping.target_account,
                mapping.target_server,
                mapping.target_character
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn format_external_package_warnings(
    warnings: &[ExternalPackageWarningResult],
    summary: &ExternalPackageSummaryResult,
) -> String {
    if warnings.is_empty() {
        return "none".to_string();
    }

    let groups = summary
        .warning_groups
        .iter()
        .map(|group| {
            format!(
                "{}/{}={}",
                format_warning_category(group.category),
                format_warning_code(group.code),
                group.count
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    let details = warnings
        .iter()
        .map(|warning| {
            format!(
                "{}/{}: {}",
                format_warning_category(warning.category),
                format_warning_code(warning.code),
                warning.source_path
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    format!(
        "{} (addon: {}, wtf: {}; groups: [{}]) [{}]",
        summary.warning_count,
        summary.addon_warning_count,
        summary.wtf_warning_count,
        groups,
        details
    )
}

fn format_warning_category(category: ExternalPackageWarningCategoryValue) -> &'static str {
    match category {
        ExternalPackageWarningCategoryValue::Addon => "addon",
        ExternalPackageWarningCategoryValue::Wtf => "wtf",
    }
}

fn format_warning_code(code: ExternalPackageWarningCodeValue) -> &'static str {
    match code {
        ExternalPackageWarningCodeValue::AddonRootNotDetected => "addon_root_not_detected",
        ExternalPackageWarningCodeValue::UnsupportedWtfLayout => "unsupported_wtf_layout",
        ExternalPackageWarningCodeValue::WtfAccountPathWithoutFile => {
            "wtf_account_path_without_file"
        }
        ExternalPackageWarningCodeValue::WtfSavedVariablesPathWithoutFile => {
            "wtf_savedvariables_path_without_file"
        }
        ExternalPackageWarningCodeValue::UnsupportedWtfNestedAccountLayout => {
            "unsupported_wtf_nested_account_layout"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_external_package_warnings;
    use crate::core::app::{
        ExternalPackageSummaryResult, ExternalPackageWarningCategoryValue,
        ExternalPackageWarningCodeValue, ExternalPackageWarningGroupResult,
        ExternalPackageWarningResult,
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
            },
        );

        assert_eq!(rendered, "none");
    }
}
