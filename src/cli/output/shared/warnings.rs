use crate::core::app::{
    ConfigPackageSummaryResult, ConfigWarningCategoryValue, ConfigWarningCodeValue,
    ConfigWarningResult, ExternalPackageSummaryResult, ExternalPackageWarningCategoryValue,
    ExternalPackageWarningCodeValue, ExternalPackageWarningResult,
};

pub(in crate::cli::output) fn format_external_package_warnings(
    warnings: &[ExternalPackageWarningResult],
    summary: &ExternalPackageSummaryResult,
) -> String {
    format_warning_report(
        warnings.is_empty(),
        summary.warning_count,
        summary.addon_warning_count,
        summary.wtf_warning_count,
        summary
            .warning_groups
            .iter()
            .map(|group| WarningGroupDisplay {
                category: format_external_package_warning_category(group.category),
                code: format_external_package_warning_code(group.code),
                count: group.count,
            })
            .collect(),
        warnings
            .iter()
            .map(|warning| WarningDetailDisplay {
                category: format_external_package_warning_category(warning.category),
                code: format_external_package_warning_code(warning.code),
                source_path: &warning.source_path,
            })
            .collect(),
    )
}

pub(in crate::cli::output) fn format_config_warnings(
    warnings: &[ConfigWarningResult],
    summary: &ConfigPackageSummaryResult,
) -> String {
    format_warning_report(
        warnings.is_empty(),
        summary.warning_count,
        summary.addon_warning_count,
        summary.wtf_warning_count,
        summary
            .warning_groups
            .iter()
            .map(|group| WarningGroupDisplay {
                category: format_config_warning_category(group.category),
                code: format_config_warning_code(group.code),
                count: group.count,
            })
            .collect(),
        warnings
            .iter()
            .map(|warning| WarningDetailDisplay {
                category: format_config_warning_category(warning.category),
                code: format_config_warning_code(warning.code),
                source_path: &warning.source_path,
            })
            .collect(),
    )
}

struct WarningGroupDisplay {
    category: &'static str,
    code: &'static str,
    count: usize,
}

struct WarningDetailDisplay<'a> {
    category: &'static str,
    code: &'static str,
    source_path: &'a str,
}

fn format_warning_report(
    is_empty: bool,
    warning_count: usize,
    addon_warning_count: usize,
    wtf_warning_count: usize,
    warning_groups: Vec<WarningGroupDisplay>,
    warning_details: Vec<WarningDetailDisplay<'_>>,
) -> String {
    if is_empty {
        return "none".to_string();
    }

    let groups = warning_groups
        .iter()
        .map(|group| format!("{}/{}={}", group.category, group.code, group.count))
        .collect::<Vec<_>>()
        .join(", ");

    let details = warning_details
        .iter()
        .map(|warning| {
            format!(
                "{}/{}: {}",
                warning.category, warning.code, warning.source_path
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");

    format!(
        "{} (addon: {}, wtf: {}; groups: [{}]) [{}]",
        warning_count, addon_warning_count, wtf_warning_count, groups, details
    )
}

fn format_external_package_warning_category(
    category: ExternalPackageWarningCategoryValue,
) -> &'static str {
    match category {
        ExternalPackageWarningCategoryValue::Addon => "addon",
        ExternalPackageWarningCategoryValue::Wtf => "wtf",
    }
}

fn format_external_package_warning_code(code: ExternalPackageWarningCodeValue) -> &'static str {
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

fn format_config_warning_category(category: ConfigWarningCategoryValue) -> &'static str {
    match category {
        ConfigWarningCategoryValue::Addon => "addon",
        ConfigWarningCategoryValue::Wtf => "wtf",
    }
}

fn format_config_warning_code(code: ConfigWarningCodeValue) -> &'static str {
    match code {
        ConfigWarningCodeValue::AddonRootNotDetected => "addon_root_not_detected",
        ConfigWarningCodeValue::UnsupportedWtfLayout => "unsupported_wtf_layout",
        ConfigWarningCodeValue::WtfAccountPathWithoutFile => "wtf_account_path_without_file",
        ConfigWarningCodeValue::WtfSavedVariablesPathWithoutFile => {
            "wtf_savedvariables_path_without_file"
        }
        ConfigWarningCodeValue::UnsupportedWtfNestedAccountLayout => {
            "unsupported_wtf_nested_account_layout"
        }
    }
}
