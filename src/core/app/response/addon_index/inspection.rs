use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::index::{
    AddonIndexIdentityHintCoverage, AddonIndexInspection,
    AddonIndexInspectionWarning as DomainAddonIndexInspectionWarning,
    AddonIndexInspectionWarningCode as DomainAddonIndexInspectionWarningCode,
    AddonIndexInspectionWarningSeverity as DomainAddonIndexInspectionWarningSeverity,
};

use super::super::super::map_owned_vec;
use super::package::AddonIndexPackageResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspectionResult {
    pub index_path: PathBuf,
    pub name: String,
    pub description: Option<String>,
    pub package_count: usize,
    pub identity_hint_coverage: AddonIndexIdentityHintCoverageResult,
    pub warning_count: usize,
    pub blocking_warning_count: usize,
    pub advisory_warning_count: usize,
    pub warnings: Vec<AddonIndexInspectionWarningResult>,
    pub packages: Vec<AddonIndexPackageResult>,
}

impl AddonIndexInspectionResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonIndexInspection, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            index_path: value.index_path,
            name: value.index.name,
            description: value.index.description,
            package_count: value.package_count,
            identity_hint_coverage: AddonIndexIdentityHintCoverageResult::from_domain(
                value.identity_hint_coverage,
            ),
            warning_count: value.warning_count,
            blocking_warning_count: value.blocking_warning_count,
            advisory_warning_count: value.advisory_warning_count,
            warnings: map_owned_vec(
                value.warnings,
                AddonIndexInspectionWarningResult::from_domain,
            ),
            packages: map_owned_vec(value.index.packages, |value| {
                AddonIndexPackageResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexValidationResult {
    pub index_path: PathBuf,
    pub name: String,
    pub package_count: usize,
    pub identity_hint_coverage: AddonIndexIdentityHintCoverageResult,
    pub valid: bool,
    pub warning_count: usize,
    pub blocking_warning_count: usize,
    pub advisory_warning_count: usize,
    pub warnings: Vec<AddonIndexInspectionWarningResult>,
}

impl AddonIndexValidationResult {
    pub(crate) fn from_inspection(value: AddonIndexInspectionResult) -> Self {
        Self {
            index_path: value.index_path,
            name: value.name,
            package_count: value.package_count,
            identity_hint_coverage: value.identity_hint_coverage,
            valid: value.blocking_warning_count == 0,
            warning_count: value.warning_count,
            blocking_warning_count: value.blocking_warning_count,
            advisory_warning_count: value.advisory_warning_count,
            warnings: value.warnings,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexInspectionWarningCodeResult {
    MissingMatchPackageIds,
    MissingAddonDirectories,
    MissingExactIdentityHints,
}

impl AddonIndexInspectionWarningCodeResult {
    fn from_domain(value: DomainAddonIndexInspectionWarningCode) -> Self {
        match value {
            DomainAddonIndexInspectionWarningCode::MissingMatchPackageIds => {
                Self::MissingMatchPackageIds
            }
            DomainAddonIndexInspectionWarningCode::MissingAddonDirectories => {
                Self::MissingAddonDirectories
            }
            DomainAddonIndexInspectionWarningCode::MissingExactIdentityHints => {
                Self::MissingExactIdentityHints
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexInspectionWarningSeverityResult {
    Blocking,
    Advisory,
}

impl AddonIndexInspectionWarningSeverityResult {
    fn from_domain(value: DomainAddonIndexInspectionWarningSeverity) -> Self {
        match value {
            DomainAddonIndexInspectionWarningSeverity::Blocking => Self::Blocking,
            DomainAddonIndexInspectionWarningSeverity::Advisory => Self::Advisory,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspectionWarningResult {
    pub code: AddonIndexInspectionWarningCodeResult,
    pub severity: AddonIndexInspectionWarningSeverityResult,
    pub package_id: String,
    pub message: String,
}

impl AddonIndexInspectionWarningResult {
    fn from_domain(value: DomainAddonIndexInspectionWarning) -> Self {
        Self {
            code: AddonIndexInspectionWarningCodeResult::from_domain(value.code),
            severity: AddonIndexInspectionWarningSeverityResult::from_domain(value.severity),
            package_id: value.package_id,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexIdentityHintCoverageResult {
    pub package_count_with_both_exact_hints: usize,
    pub package_count_with_any_exact_hints: usize,
    pub package_count_with_match_package_ids: usize,
    pub package_count_with_addon_directories: usize,
    pub package_count_without_match_package_ids: usize,
    pub package_count_without_addon_directories: usize,
    pub package_count_without_exact_hints: usize,
    pub packages_without_match_package_ids: Vec<String>,
    pub packages_without_addon_directories: Vec<String>,
    pub packages_without_exact_hints: Vec<String>,
}

impl AddonIndexIdentityHintCoverageResult {
    fn from_domain(value: AddonIndexIdentityHintCoverage) -> Self {
        Self {
            package_count_with_both_exact_hints: value.package_count_with_both_exact_hints,
            package_count_with_any_exact_hints: value.package_count_with_any_exact_hints,
            package_count_with_match_package_ids: value.package_count_with_match_package_ids,
            package_count_with_addon_directories: value.package_count_with_addon_directories,
            package_count_without_match_package_ids: value.package_count_without_match_package_ids,
            package_count_without_addon_directories: value.package_count_without_addon_directories,
            package_count_without_exact_hints: value.package_count_without_exact_hints,
            packages_without_match_package_ids: value.packages_without_match_package_ids,
            packages_without_addon_directories: value.packages_without_addon_directories,
            packages_without_exact_hints: value.packages_without_exact_hints,
        }
    }
}
