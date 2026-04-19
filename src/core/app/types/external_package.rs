use serde::{Deserialize, Serialize};

use crate::core::bundle::{
    ExternalPackageWarningCategory as DomainExternalPackageWarningCategory,
    ExternalPackageWarningCode as DomainExternalPackageWarningCode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCategoryValue {
    Addon,
    Wtf,
}

impl From<DomainExternalPackageWarningCategory> for ExternalPackageWarningCategoryValue {
    fn from(value: DomainExternalPackageWarningCategory) -> Self {
        match value {
            DomainExternalPackageWarningCategory::Addon => Self::Addon,
            DomainExternalPackageWarningCategory::Wtf => Self::Wtf,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCodeValue {
    AddonRootNotDetected,
    UnsupportedWtfLayout,
    UnsupportedWtfRootSavedVariables,
    WtfAccountPathWithoutFile,
    WtfSavedVariablesPathWithoutFile,
    UnsupportedWtfNestedAccountLayout,
}

impl From<DomainExternalPackageWarningCode> for ExternalPackageWarningCodeValue {
    fn from(value: DomainExternalPackageWarningCode) -> Self {
        match value {
            DomainExternalPackageWarningCode::AddonRootNotDetected => Self::AddonRootNotDetected,
            DomainExternalPackageWarningCode::UnsupportedWtfLayout => Self::UnsupportedWtfLayout,
            DomainExternalPackageWarningCode::UnsupportedWtfRootSavedVariables => {
                Self::UnsupportedWtfRootSavedVariables
            }
            DomainExternalPackageWarningCode::WtfAccountPathWithoutFile => {
                Self::WtfAccountPathWithoutFile
            }
            DomainExternalPackageWarningCode::WtfSavedVariablesPathWithoutFile => {
                Self::WtfSavedVariablesPathWithoutFile
            }
            DomainExternalPackageWarningCode::UnsupportedWtfNestedAccountLayout => {
                Self::UnsupportedWtfNestedAccountLayout
            }
        }
    }
}
