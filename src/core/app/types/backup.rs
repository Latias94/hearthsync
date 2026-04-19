use serde::{Deserialize, Serialize};

use crate::core::backup::BackupGroup as DomainBackupGroup;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupGroupValue {
    Addons,
    Wtf,
    Fonts,
    InterfaceAssets,
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
