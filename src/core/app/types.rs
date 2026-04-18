use serde::{Deserialize, Serialize};

use crate::core::backup::BackupGroup as DomainBackupGroup;
use crate::core::bundle::{
    ApplyAction as DomainApplyAction, ApplyGroup as DomainApplyGroup,
    HelperStrategy as DomainHelperStrategy, WtfScope as DomainWtfScope,
};
use crate::core::manifest::ResourceApplyPolicy as DomainResourceApplyPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupGroupValue {
    Addons,
    Wtf,
    Fonts,
    InterfaceAssets,
}

impl BackupGroupValue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Addons => "addons",
            Self::Wtf => "wtf",
            Self::Fonts => "fonts",
            Self::InterfaceAssets => "interface_assets",
        }
    }
}

impl From<DomainBackupGroup> for BackupGroupValue {
    fn from(value: DomainBackupGroup) -> Self {
        match value {
            DomainBackupGroup::Addons => Self::Addons,
            DomainBackupGroup::Wtf => Self::Wtf,
            DomainBackupGroup::Fonts => Self::Fonts,
            DomainBackupGroup::InterfaceAssets => Self::InterfaceAssets,
        }
    }
}

impl From<BackupGroupValue> for DomainBackupGroup {
    fn from(value: BackupGroupValue) -> Self {
        match value {
            BackupGroupValue::Addons => Self::Addons,
            BackupGroupValue::Wtf => Self::Wtf,
            BackupGroupValue::Fonts => Self::Fonts,
            BackupGroupValue::InterfaceAssets => Self::InterfaceAssets,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyActionValue {
    Remove,
    Add,
    Replace,
    Skip,
    Preserve,
}

impl From<DomainApplyAction> for ApplyActionValue {
    fn from(value: DomainApplyAction) -> Self {
        match value {
            DomainApplyAction::Remove => Self::Remove,
            DomainApplyAction::Add => Self::Add,
            DomainApplyAction::Replace => Self::Replace,
            DomainApplyAction::Skip => Self::Skip,
            DomainApplyAction::Preserve => Self::Preserve,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApplyGroupValue {
    Addons,
    WtfCommon,
    WtfCharacters,
    Fonts,
    InterfaceAssets,
    Metadata,
}

impl From<DomainApplyGroup> for ApplyGroupValue {
    fn from(value: DomainApplyGroup) -> Self {
        match value {
            DomainApplyGroup::Addons => Self::Addons,
            DomainApplyGroup::WtfCommon => Self::WtfCommon,
            DomainApplyGroup::WtfCharacters => Self::WtfCharacters,
            DomainApplyGroup::Fonts => Self::Fonts,
            DomainApplyGroup::InterfaceAssets => Self::InterfaceAssets,
            DomainApplyGroup::Metadata => Self::Metadata,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WtfScopeValue {
    GlobalConfig,
    RootSavedVariables,
    AccountRootFile,
    AccountSavedVariables,
    CharacterSavedVariables,
    CharacterState,
    CacheLike,
    Unknown,
}

impl From<DomainWtfScope> for WtfScopeValue {
    fn from(value: DomainWtfScope) -> Self {
        match value {
            DomainWtfScope::GlobalConfig => Self::GlobalConfig,
            DomainWtfScope::RootSavedVariables => Self::RootSavedVariables,
            DomainWtfScope::AccountRootFile => Self::AccountRootFile,
            DomainWtfScope::AccountSavedVariables => Self::AccountSavedVariables,
            DomainWtfScope::CharacterSavedVariables => Self::CharacterSavedVariables,
            DomainWtfScope::CharacterState => Self::CharacterState,
            DomainWtfScope::CacheLike => Self::CacheLike,
            DomainWtfScope::Unknown => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperStrategyValue {
    NativeRust,
}

impl From<DomainHelperStrategy> for HelperStrategyValue {
    fn from(value: DomainHelperStrategy) -> Self {
        match value {
            DomainHelperStrategy::NativeRust => Self::NativeRust,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceApplyPolicyValue {
    Merge,
    Share,
    Sync,
    Mirror,
    ReplaceSelected,
    Preserve,
}

impl From<DomainResourceApplyPolicy> for ResourceApplyPolicyValue {
    fn from(value: DomainResourceApplyPolicy) -> Self {
        match value {
            DomainResourceApplyPolicy::Merge => Self::Merge,
            DomainResourceApplyPolicy::Share => Self::Share,
            DomainResourceApplyPolicy::Sync => Self::Sync,
            DomainResourceApplyPolicy::Mirror => Self::Mirror,
            DomainResourceApplyPolicy::ReplaceSelected => Self::ReplaceSelected,
            DomainResourceApplyPolicy::Preserve => Self::Preserve,
        }
    }
}
