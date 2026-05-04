use serde::{Deserialize, Serialize};

use crate::core::bundle::{
    ExternalPackageLayout as DomainExternalPackageLayout,
    ExternalPackageWarningCategory as DomainExternalPackageWarningCategory,
    ExternalPackageWarningCode as DomainExternalPackageWarningCode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageLayoutValue {
    Auto,
    Generic,
    #[serde(rename = "newbeebox_addon")]
    NewBeeBoxAddon,
    #[serde(rename = "newbeebox_font")]
    NewBeeBoxFont,
    #[serde(rename = "newbeebox_material")]
    NewBeeBoxMaterial,
    #[serde(rename = "newbeebox_wtf_account")]
    NewBeeBoxWtfAccount,
    #[serde(rename = "newbeebox_wtf_character")]
    NewBeeBoxWtfCharacter,
}

impl Default for ExternalPackageLayoutValue {
    fn default() -> Self {
        Self::Auto
    }
}

impl ExternalPackageLayoutValue {
    pub(crate) fn into_domain(self) -> DomainExternalPackageLayout {
        match self {
            Self::Auto => DomainExternalPackageLayout::Auto,
            Self::Generic => DomainExternalPackageLayout::Generic,
            Self::NewBeeBoxAddon => DomainExternalPackageLayout::NewBeeBoxAddon,
            Self::NewBeeBoxFont => DomainExternalPackageLayout::NewBeeBoxFont,
            Self::NewBeeBoxMaterial => DomainExternalPackageLayout::NewBeeBoxMaterial,
            Self::NewBeeBoxWtfAccount => DomainExternalPackageLayout::NewBeeBoxWtfAccount,
            Self::NewBeeBoxWtfCharacter => DomainExternalPackageLayout::NewBeeBoxWtfCharacter,
        }
    }

    pub(crate) fn from_domain(value: DomainExternalPackageLayout) -> Self {
        match value {
            DomainExternalPackageLayout::Auto => Self::Auto,
            DomainExternalPackageLayout::Generic => Self::Generic,
            DomainExternalPackageLayout::NewBeeBoxAddon => Self::NewBeeBoxAddon,
            DomainExternalPackageLayout::NewBeeBoxFont => Self::NewBeeBoxFont,
            DomainExternalPackageLayout::NewBeeBoxMaterial => Self::NewBeeBoxMaterial,
            DomainExternalPackageLayout::NewBeeBoxWtfAccount => Self::NewBeeBoxWtfAccount,
            DomainExternalPackageLayout::NewBeeBoxWtfCharacter => Self::NewBeeBoxWtfCharacter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalPackageWarningCategoryValue {
    Addon,
    Wtf,
}

impl ExternalPackageWarningCategoryValue {
    pub(crate) fn from_domain(value: DomainExternalPackageWarningCategory) -> Self {
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
    WtfAccountPathWithoutFile,
    WtfSavedVariablesPathWithoutFile,
    UnsupportedWtfNestedAccountLayout,
}

impl ExternalPackageWarningCodeValue {
    pub(crate) fn from_domain(value: DomainExternalPackageWarningCode) -> Self {
        match value {
            DomainExternalPackageWarningCode::AddonRootNotDetected => Self::AddonRootNotDetected,
            DomainExternalPackageWarningCode::UnsupportedWtfLayout => Self::UnsupportedWtfLayout,
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
