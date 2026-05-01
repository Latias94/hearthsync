use serde::Serialize;

use crate::core::app::{ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue};
use crate::core::bundle::{
    ExternalPackageSummary as DomainExternalPackageSummary,
    ExternalPackageWarning as DomainExternalPackageWarning,
    ExternalPackageWarningGroup as DomainExternalPackageWarningGroup,
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
        }
    }
}
