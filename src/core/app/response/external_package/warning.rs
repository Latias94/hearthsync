use serde::Serialize;

use crate::core::app::{
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue, WtfScopeRiskValue,
    WtfScopeValue,
};
use crate::core::bundle::{
    ExternalPackagePublicSharingReason as DomainExternalPackagePublicSharingReason,
    ExternalPackagePublicSharingReasonCode as DomainExternalPackagePublicSharingReasonCode,
    ExternalPackagePublicSharingSeverity as DomainExternalPackagePublicSharingSeverity,
    ExternalPackagePublicSharingStatus as DomainExternalPackagePublicSharingStatus,
    ExternalPackagePublicSharingSummary as DomainExternalPackagePublicSharingSummary,
    ExternalPackageSourceCharacterSummary as DomainExternalPackageSourceCharacterSummary,
    ExternalPackageSourceIdentitySummary as DomainExternalPackageSourceIdentitySummary,
    ExternalPackageSummary as DomainExternalPackageSummary,
    ExternalPackageWarning as DomainExternalPackageWarning,
    ExternalPackageWarningGroup as DomainExternalPackageWarningGroup,
    ExternalPackageWtfScopeSummary as DomainExternalPackageWtfScopeSummary,
};

use super::super::super::map_owned_vec;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageWarningGroupResult {
    pub category: ExternalPackageWarningCategoryValue,
    pub code: ExternalPackageWarningCodeValue,
    pub count: usize,
}

impl ExternalPackageWarningGroupResult {
    pub(crate) fn from_domain(value: DomainExternalPackageWarningGroup) -> Self {
        Self {
            category: ExternalPackageWarningCategoryValue::from_domain(value.category),
            code: ExternalPackageWarningCodeValue::from_domain(value.code),
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageWarningResult {
    pub category: ExternalPackageWarningCategoryValue,
    pub code: ExternalPackageWarningCodeValue,
    pub source_path: String,
    pub message: String,
}

impl ExternalPackageWarningResult {
    pub(crate) fn from_domain(value: DomainExternalPackageWarning) -> Self {
        Self {
            category: ExternalPackageWarningCategoryValue::from_domain(value.category),
            code: ExternalPackageWarningCodeValue::from_domain(value.code),
            source_path: value.source_path,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageWtfScopeSummaryResult {
    pub scope: WtfScopeValue,
    pub risk: WtfScopeRiskValue,
    pub count: usize,
}

impl ExternalPackageWtfScopeSummaryResult {
    pub(crate) fn from_domain(value: DomainExternalPackageWtfScopeSummary) -> Self {
        Self {
            scope: WtfScopeValue::from_domain(value.scope),
            risk: WtfScopeRiskValue::from_domain(value.risk),
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackagePublicSharingStatusValue {
    Ready,
    Advisory,
    ReviewRequired,
}

impl ExternalPackagePublicSharingStatusValue {
    pub(crate) fn from_domain(value: DomainExternalPackagePublicSharingStatus) -> Self {
        match value {
            DomainExternalPackagePublicSharingStatus::Ready => Self::Ready,
            DomainExternalPackagePublicSharingStatus::Advisory => Self::Advisory,
            DomainExternalPackagePublicSharingStatus::ReviewRequired => Self::ReviewRequired,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackagePublicSharingSeverityValue {
    Advisory,
    ReviewRequired,
}

impl ExternalPackagePublicSharingSeverityValue {
    pub(crate) fn from_domain(value: DomainExternalPackagePublicSharingSeverity) -> Self {
        match value {
            DomainExternalPackagePublicSharingSeverity::Advisory => Self::Advisory,
            DomainExternalPackagePublicSharingSeverity::ReviewRequired => Self::ReviewRequired,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackagePublicSharingReasonCodeValue {
    NormalizationWarnings,
    HighRiskWtfScope,
    MediumRiskWtfScope,
    LowRiskWtfScope,
    UnknownRiskWtfScope,
    SourceAccountIdentity,
    SourceCharacterIdentity,
}

impl ExternalPackagePublicSharingReasonCodeValue {
    pub(crate) fn from_domain(value: DomainExternalPackagePublicSharingReasonCode) -> Self {
        match value {
            DomainExternalPackagePublicSharingReasonCode::NormalizationWarnings => {
                Self::NormalizationWarnings
            }
            DomainExternalPackagePublicSharingReasonCode::HighRiskWtfScope => {
                Self::HighRiskWtfScope
            }
            DomainExternalPackagePublicSharingReasonCode::MediumRiskWtfScope => {
                Self::MediumRiskWtfScope
            }
            DomainExternalPackagePublicSharingReasonCode::LowRiskWtfScope => Self::LowRiskWtfScope,
            DomainExternalPackagePublicSharingReasonCode::UnknownRiskWtfScope => {
                Self::UnknownRiskWtfScope
            }
            DomainExternalPackagePublicSharingReasonCode::SourceAccountIdentity => {
                Self::SourceAccountIdentity
            }
            DomainExternalPackagePublicSharingReasonCode::SourceCharacterIdentity => {
                Self::SourceCharacterIdentity
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackagePublicSharingReasonResult {
    pub severity: ExternalPackagePublicSharingSeverityValue,
    pub code: ExternalPackagePublicSharingReasonCodeValue,
    pub count: usize,
    pub message: String,
}

impl ExternalPackagePublicSharingReasonResult {
    pub(crate) fn from_domain(value: DomainExternalPackagePublicSharingReason) -> Self {
        Self {
            severity: ExternalPackagePublicSharingSeverityValue::from_domain(value.severity),
            code: ExternalPackagePublicSharingReasonCodeValue::from_domain(value.code),
            count: value.count,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackagePublicSharingSummaryResult {
    pub status: ExternalPackagePublicSharingStatusValue,
    pub public_ready: bool,
    pub review_required_count: usize,
    pub advisory_count: usize,
    pub reasons: Vec<ExternalPackagePublicSharingReasonResult>,
}

impl ExternalPackagePublicSharingSummaryResult {
    pub(crate) fn from_domain(value: DomainExternalPackagePublicSharingSummary) -> Self {
        Self {
            status: ExternalPackagePublicSharingStatusValue::from_domain(value.status),
            public_ready: value.public_ready,
            review_required_count: value.review_required_count,
            advisory_count: value.advisory_count,
            reasons: value
                .reasons
                .into_iter()
                .map(ExternalPackagePublicSharingReasonResult::from_domain)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageSourceCharacterResult {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
}

impl ExternalPackageSourceCharacterResult {
    pub(crate) fn from_domain(value: DomainExternalPackageSourceCharacterSummary) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageSourceIdentityResult {
    pub source_accounts: Vec<String>,
    pub source_characters: Vec<ExternalPackageSourceCharacterResult>,
    pub entries_with_source_account: usize,
    pub entries_with_source_character: usize,
}

impl ExternalPackageSourceIdentityResult {
    pub(crate) fn from_domain(value: DomainExternalPackageSourceIdentitySummary) -> Self {
        Self {
            source_accounts: value.source_accounts,
            source_characters: value
                .source_characters
                .into_iter()
                .map(ExternalPackageSourceCharacterResult::from_domain)
                .collect(),
            entries_with_source_account: value.entries_with_source_account,
            entries_with_source_character: value.entries_with_source_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageSummaryResult {
    pub total_files: usize,
    pub normalized_files: usize,
    pub ignored_files: usize,
    pub addons: usize,
    pub wtf_common: usize,
    pub wtf_characters: usize,
    pub fonts: usize,
    pub interface_assets: usize,
    pub warning_count: usize,
    pub addon_warning_count: usize,
    pub wtf_warning_count: usize,
    pub warning_groups: Vec<ExternalPackageWarningGroupResult>,
    pub wtf_scopes: Vec<ExternalPackageWtfScopeSummaryResult>,
    pub source_identities: ExternalPackageSourceIdentityResult,
    pub public_sharing: ExternalPackagePublicSharingSummaryResult,
}

impl ExternalPackageSummaryResult {
    pub(crate) fn from_domain(value: DomainExternalPackageSummary) -> Self {
        Self {
            total_files: value.total_files,
            normalized_files: value.normalized_files,
            ignored_files: value.ignored_files,
            addons: value.addons,
            wtf_common: value.wtf_common,
            wtf_characters: value.wtf_characters,
            fonts: value.fonts,
            interface_assets: value.interface_assets,
            warning_count: value.warning_count,
            addon_warning_count: value.addon_warning_count,
            wtf_warning_count: value.wtf_warning_count,
            warning_groups: map_owned_vec(
                value.warning_groups,
                ExternalPackageWarningGroupResult::from_domain,
            ),
            wtf_scopes: map_owned_vec(
                value.wtf_scopes,
                ExternalPackageWtfScopeSummaryResult::from_domain,
            ),
            source_identities: ExternalPackageSourceIdentityResult::from_domain(
                value.source_identities,
            ),
            public_sharing: ExternalPackagePublicSharingSummaryResult::from_domain(
                value.public_sharing,
            ),
        }
    }
}
