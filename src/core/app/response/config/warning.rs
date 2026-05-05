use serde::Serialize;

use crate::core::app::types::external_package::{
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
};
use crate::core::app::{
    ExternalPackageSourceCharacterResult, ExternalPackageSourceIdentityResult, WtfScopeRiskValue,
    WtfScopeValue,
};

use super::super::external_package::{
    ExternalPackagePublicSharingReasonCodeValue, ExternalPackagePublicSharingReasonResult,
    ExternalPackagePublicSharingSeverityValue, ExternalPackagePublicSharingStatusValue,
    ExternalPackagePublicSharingSummaryResult, ExternalPackageSummaryResult,
    ExternalPackageWarningGroupResult, ExternalPackageWarningResult,
    ExternalPackageWtfScopeSummaryResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigWarningCategoryValue {
    Addon,
    Wtf,
}

impl ConfigWarningCategoryValue {
    fn from_external(value: ExternalPackageWarningCategoryValue) -> Self {
        match value {
            ExternalPackageWarningCategoryValue::Addon => Self::Addon,
            ExternalPackageWarningCategoryValue::Wtf => Self::Wtf,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigWarningCodeValue {
    AddonRootNotDetected,
    UnsupportedWtfLayout,
    WtfAccountPathWithoutFile,
    WtfSavedVariablesPathWithoutFile,
    UnsupportedWtfNestedAccountLayout,
}

impl ConfigWarningCodeValue {
    fn from_external(value: ExternalPackageWarningCodeValue) -> Self {
        match value {
            ExternalPackageWarningCodeValue::AddonRootNotDetected => Self::AddonRootNotDetected,
            ExternalPackageWarningCodeValue::UnsupportedWtfLayout => Self::UnsupportedWtfLayout,
            ExternalPackageWarningCodeValue::WtfAccountPathWithoutFile => {
                Self::WtfAccountPathWithoutFile
            }
            ExternalPackageWarningCodeValue::WtfSavedVariablesPathWithoutFile => {
                Self::WtfSavedVariablesPathWithoutFile
            }
            ExternalPackageWarningCodeValue::UnsupportedWtfNestedAccountLayout => {
                Self::UnsupportedWtfNestedAccountLayout
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigWarningGroupResult {
    pub category: ConfigWarningCategoryValue,
    pub code: ConfigWarningCodeValue,
    pub count: usize,
}

impl ConfigWarningGroupResult {
    fn from_external(value: ExternalPackageWarningGroupResult) -> Self {
        Self {
            category: ConfigWarningCategoryValue::from_external(value.category),
            code: ConfigWarningCodeValue::from_external(value.code),
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigWarningResult {
    pub category: ConfigWarningCategoryValue,
    pub code: ConfigWarningCodeValue,
    pub source_path: String,
    pub message: String,
}

impl ConfigWarningResult {
    pub(super) fn from_external(value: ExternalPackageWarningResult) -> Self {
        Self {
            category: ConfigWarningCategoryValue::from_external(value.category),
            code: ConfigWarningCodeValue::from_external(value.code),
            source_path: value.source_path,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigWtfScopeSummaryResult {
    pub scope: WtfScopeValue,
    pub risk: WtfScopeRiskValue,
    pub count: usize,
}

impl ConfigWtfScopeSummaryResult {
    fn from_external(value: ExternalPackageWtfScopeSummaryResult) -> Self {
        Self {
            scope: value.scope,
            risk: value.risk,
            count: value.count,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPublicSharingStatusValue {
    Ready,
    Advisory,
    ReviewRequired,
}

impl ConfigPublicSharingStatusValue {
    fn from_external(value: ExternalPackagePublicSharingStatusValue) -> Self {
        match value {
            ExternalPackagePublicSharingStatusValue::Ready => Self::Ready,
            ExternalPackagePublicSharingStatusValue::Advisory => Self::Advisory,
            ExternalPackagePublicSharingStatusValue::ReviewRequired => Self::ReviewRequired,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPublicSharingSeverityValue {
    Advisory,
    ReviewRequired,
}

impl ConfigPublicSharingSeverityValue {
    fn from_external(value: ExternalPackagePublicSharingSeverityValue) -> Self {
        match value {
            ExternalPackagePublicSharingSeverityValue::Advisory => Self::Advisory,
            ExternalPackagePublicSharingSeverityValue::ReviewRequired => Self::ReviewRequired,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPublicSharingReasonCodeValue {
    NormalizationWarnings,
    HighRiskWtfScope,
    MediumRiskWtfScope,
    LowRiskWtfScope,
    UnknownRiskWtfScope,
    SourceAccountIdentity,
    SourceCharacterIdentity,
}

impl ConfigPublicSharingReasonCodeValue {
    fn from_external(value: ExternalPackagePublicSharingReasonCodeValue) -> Self {
        match value {
            ExternalPackagePublicSharingReasonCodeValue::NormalizationWarnings => {
                Self::NormalizationWarnings
            }
            ExternalPackagePublicSharingReasonCodeValue::HighRiskWtfScope => Self::HighRiskWtfScope,
            ExternalPackagePublicSharingReasonCodeValue::MediumRiskWtfScope => {
                Self::MediumRiskWtfScope
            }
            ExternalPackagePublicSharingReasonCodeValue::LowRiskWtfScope => Self::LowRiskWtfScope,
            ExternalPackagePublicSharingReasonCodeValue::UnknownRiskWtfScope => {
                Self::UnknownRiskWtfScope
            }
            ExternalPackagePublicSharingReasonCodeValue::SourceAccountIdentity => {
                Self::SourceAccountIdentity
            }
            ExternalPackagePublicSharingReasonCodeValue::SourceCharacterIdentity => {
                Self::SourceCharacterIdentity
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPublicSharingReasonResult {
    pub severity: ConfigPublicSharingSeverityValue,
    pub code: ConfigPublicSharingReasonCodeValue,
    pub count: usize,
    pub message: String,
}

impl ConfigPublicSharingReasonResult {
    fn from_external(value: ExternalPackagePublicSharingReasonResult) -> Self {
        Self {
            severity: ConfigPublicSharingSeverityValue::from_external(value.severity),
            code: ConfigPublicSharingReasonCodeValue::from_external(value.code),
            count: value.count,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPublicSharingSummaryResult {
    pub status: ConfigPublicSharingStatusValue,
    pub public_ready: bool,
    pub review_required_count: usize,
    pub advisory_count: usize,
    pub reasons: Vec<ConfigPublicSharingReasonResult>,
}

impl ConfigPublicSharingSummaryResult {
    fn from_external(value: ExternalPackagePublicSharingSummaryResult) -> Self {
        Self {
            status: ConfigPublicSharingStatusValue::from_external(value.status),
            public_ready: value.public_ready,
            review_required_count: value.review_required_count,
            advisory_count: value.advisory_count,
            reasons: value
                .reasons
                .into_iter()
                .map(ConfigPublicSharingReasonResult::from_external)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSourceCharacterResult {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
}

impl ConfigSourceCharacterResult {
    fn from_external(value: ExternalPackageSourceCharacterResult) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSourceIdentityResult {
    pub source_accounts: Vec<String>,
    pub source_characters: Vec<ConfigSourceCharacterResult>,
    pub entries_with_source_account: usize,
    pub entries_with_source_character: usize,
}

impl ConfigSourceIdentityResult {
    fn from_external(value: ExternalPackageSourceIdentityResult) -> Self {
        Self {
            source_accounts: value.source_accounts,
            source_characters: value
                .source_characters
                .into_iter()
                .map(ConfigSourceCharacterResult::from_external)
                .collect(),
            entries_with_source_account: value.entries_with_source_account,
            entries_with_source_character: value.entries_with_source_character,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPackageSummaryResult {
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
    pub warning_groups: Vec<ConfigWarningGroupResult>,
    pub wtf_scopes: Vec<ConfigWtfScopeSummaryResult>,
    pub source_identities: ConfigSourceIdentityResult,
    pub public_sharing: ConfigPublicSharingSummaryResult,
}

impl ConfigPackageSummaryResult {
    pub(super) fn from_external(value: ExternalPackageSummaryResult) -> Self {
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
            warning_groups: value
                .warning_groups
                .into_iter()
                .map(ConfigWarningGroupResult::from_external)
                .collect(),
            wtf_scopes: value
                .wtf_scopes
                .into_iter()
                .map(ConfigWtfScopeSummaryResult::from_external)
                .collect(),
            source_identities: ConfigSourceIdentityResult::from_external(value.source_identities),
            public_sharing: ConfigPublicSharingSummaryResult::from_external(value.public_sharing),
        }
    }
}
