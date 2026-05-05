use crate::core::app::{
    ConfigPublicSharingSeverityValue, ConfigSensitiveWtfFileKindValue,
    ConfigSensitiveWtfFileSummaryResult, ExternalPackagePublicSharingSeverityValue,
    ExternalPackageSensitiveWtfFileKindValue, ExternalPackageSensitiveWtfFileSummaryResult,
};

pub(in crate::cli::output) fn format_external_package_sensitive_wtf_files(
    files: &[ExternalPackageSensitiveWtfFileSummaryResult],
) -> String {
    format_sensitive_wtf_files(files.iter().map(|file| SensitiveWtfFileDisplay {
        kind: format_external_kind(file.kind),
        severity: format_external_severity(file.severity),
        count: file.count,
    }))
}

pub(in crate::cli::output) fn format_config_sensitive_wtf_files(
    files: &[ConfigSensitiveWtfFileSummaryResult],
) -> String {
    format_sensitive_wtf_files(files.iter().map(|file| SensitiveWtfFileDisplay {
        kind: format_config_kind(file.kind),
        severity: format_config_severity(file.severity),
        count: file.count,
    }))
}

struct SensitiveWtfFileDisplay {
    kind: &'static str,
    severity: &'static str,
    count: usize,
}

fn format_sensitive_wtf_files(files: impl IntoIterator<Item = SensitiveWtfFileDisplay>) -> String {
    let entries = files
        .into_iter()
        .map(|file| format!("{}({})={}", file.kind, file.severity, file.count))
        .collect::<Vec<_>>();

    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

fn format_external_kind(kind: ExternalPackageSensitiveWtfFileKindValue) -> &'static str {
    match kind {
        ExternalPackageSensitiveWtfFileKindValue::SavedVariables => "saved_variables",
        ExternalPackageSensitiveWtfFileKindValue::ChatCache => "chat_cache",
        ExternalPackageSensitiveWtfFileKindValue::Macros => "macros",
        ExternalPackageSensitiveWtfFileKindValue::Bindings => "bindings",
        ExternalPackageSensitiveWtfFileKindValue::GameConfig => "game_config",
        ExternalPackageSensitiveWtfFileKindValue::AddonEnablement => "addon_enablement",
        ExternalPackageSensitiveWtfFileKindValue::LayoutState => "layout_state",
    }
}

fn format_config_kind(kind: ConfigSensitiveWtfFileKindValue) -> &'static str {
    match kind {
        ConfigSensitiveWtfFileKindValue::SavedVariables => "saved_variables",
        ConfigSensitiveWtfFileKindValue::ChatCache => "chat_cache",
        ConfigSensitiveWtfFileKindValue::Macros => "macros",
        ConfigSensitiveWtfFileKindValue::Bindings => "bindings",
        ConfigSensitiveWtfFileKindValue::GameConfig => "game_config",
        ConfigSensitiveWtfFileKindValue::AddonEnablement => "addon_enablement",
        ConfigSensitiveWtfFileKindValue::LayoutState => "layout_state",
    }
}

fn format_external_severity(severity: ExternalPackagePublicSharingSeverityValue) -> &'static str {
    match severity {
        ExternalPackagePublicSharingSeverityValue::Advisory => "advisory",
        ExternalPackagePublicSharingSeverityValue::ReviewRequired => "review_required",
    }
}

fn format_config_severity(severity: ConfigPublicSharingSeverityValue) -> &'static str {
    match severity {
        ConfigPublicSharingSeverityValue::Advisory => "advisory",
        ConfigPublicSharingSeverityValue::ReviewRequired => "review_required",
    }
}
