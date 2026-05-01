use serde::Serialize;

use crate::core::app::types::external_package::{
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
};

use super::super::external_package::{
    ExternalPackageSummaryResult, ExternalPackageWarningGroupResult, ExternalPackageWarningResult,
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
        }
    }
}
