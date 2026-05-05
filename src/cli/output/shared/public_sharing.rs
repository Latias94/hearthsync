use crate::core::app::{
    ConfigPublicSharingReasonCodeValue, ConfigPublicSharingSeverityValue,
    ConfigPublicSharingStatusValue, ConfigPublicSharingSummaryResult,
    ExternalPackagePublicSharingReasonCodeValue, ExternalPackagePublicSharingSeverityValue,
    ExternalPackagePublicSharingStatusValue, ExternalPackagePublicSharingSummaryResult,
};

pub(in crate::cli::output) fn format_external_package_public_sharing(
    sharing: &ExternalPackagePublicSharingSummaryResult,
) -> String {
    format_public_sharing(PublicSharingDisplay {
        status: format_external_status(sharing.status),
        public_ready: sharing.public_ready,
        review_required_count: sharing.review_required_count,
        advisory_count: sharing.advisory_count,
        reasons: sharing
            .reasons
            .iter()
            .map(|reason| PublicSharingReasonDisplay {
                severity: format_external_severity(reason.severity),
                code: format_external_reason_code(reason.code),
                count: reason.count,
            })
            .collect(),
    })
}

pub(in crate::cli::output) fn format_config_public_sharing(
    sharing: &ConfigPublicSharingSummaryResult,
) -> String {
    format_public_sharing(PublicSharingDisplay {
        status: format_config_status(sharing.status),
        public_ready: sharing.public_ready,
        review_required_count: sharing.review_required_count,
        advisory_count: sharing.advisory_count,
        reasons: sharing
            .reasons
            .iter()
            .map(|reason| PublicSharingReasonDisplay {
                severity: format_config_severity(reason.severity),
                code: format_config_reason_code(reason.code),
                count: reason.count,
            })
            .collect(),
    })
}

struct PublicSharingDisplay {
    status: &'static str,
    public_ready: bool,
    review_required_count: usize,
    advisory_count: usize,
    reasons: Vec<PublicSharingReasonDisplay>,
}

struct PublicSharingReasonDisplay {
    severity: &'static str,
    code: &'static str,
    count: usize,
}

fn format_public_sharing(sharing: PublicSharingDisplay) -> String {
    let ready = if sharing.public_ready { "yes" } else { "no" };

    if sharing.reasons.is_empty() {
        return format!("{} (ready: {})", sharing.status, ready);
    }

    let reasons = sharing
        .reasons
        .into_iter()
        .map(|reason| format!("{}/{}={}", reason.severity, reason.code, reason.count))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "{} (ready: {}, review: {}, advisory: {}; reasons: {})",
        sharing.status, ready, sharing.review_required_count, sharing.advisory_count, reasons
    )
}

fn format_external_status(status: ExternalPackagePublicSharingStatusValue) -> &'static str {
    match status {
        ExternalPackagePublicSharingStatusValue::Ready => "ready",
        ExternalPackagePublicSharingStatusValue::Advisory => "advisory",
        ExternalPackagePublicSharingStatusValue::ReviewRequired => "review_required",
    }
}

fn format_config_status(status: ConfigPublicSharingStatusValue) -> &'static str {
    match status {
        ConfigPublicSharingStatusValue::Ready => "ready",
        ConfigPublicSharingStatusValue::Advisory => "advisory",
        ConfigPublicSharingStatusValue::ReviewRequired => "review_required",
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

fn format_external_reason_code(code: ExternalPackagePublicSharingReasonCodeValue) -> &'static str {
    match code {
        ExternalPackagePublicSharingReasonCodeValue::NormalizationWarnings => {
            "normalization_warnings"
        }
        ExternalPackagePublicSharingReasonCodeValue::HighRiskWtfScope => "high_risk_wtf_scope",
        ExternalPackagePublicSharingReasonCodeValue::MediumRiskWtfScope => "medium_risk_wtf_scope",
        ExternalPackagePublicSharingReasonCodeValue::LowRiskWtfScope => "low_risk_wtf_scope",
        ExternalPackagePublicSharingReasonCodeValue::UnknownRiskWtfScope => {
            "unknown_risk_wtf_scope"
        }
        ExternalPackagePublicSharingReasonCodeValue::SourceAccountIdentity => {
            "source_account_identity"
        }
        ExternalPackagePublicSharingReasonCodeValue::SourceCharacterIdentity => {
            "source_character_identity"
        }
    }
}

fn format_config_reason_code(code: ConfigPublicSharingReasonCodeValue) -> &'static str {
    match code {
        ConfigPublicSharingReasonCodeValue::NormalizationWarnings => "normalization_warnings",
        ConfigPublicSharingReasonCodeValue::HighRiskWtfScope => "high_risk_wtf_scope",
        ConfigPublicSharingReasonCodeValue::MediumRiskWtfScope => "medium_risk_wtf_scope",
        ConfigPublicSharingReasonCodeValue::LowRiskWtfScope => "low_risk_wtf_scope",
        ConfigPublicSharingReasonCodeValue::UnknownRiskWtfScope => "unknown_risk_wtf_scope",
        ConfigPublicSharingReasonCodeValue::SourceAccountIdentity => "source_account_identity",
        ConfigPublicSharingReasonCodeValue::SourceCharacterIdentity => "source_character_identity",
    }
}
