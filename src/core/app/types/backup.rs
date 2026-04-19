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

impl BackupGroupValue {
    pub(crate) fn from_domain(value: DomainBackupGroup) -> Self {
        match value {
            DomainBackupGroup::Addons => Self::Addons,
            DomainBackupGroup::Wtf => Self::Wtf,
            DomainBackupGroup::Fonts => Self::Fonts,
            DomainBackupGroup::InterfaceAssets => Self::InterfaceAssets,
        }
    }

    pub(crate) fn into_domain(self) -> DomainBackupGroup {
        match self {
            Self::Addons => DomainBackupGroup::Addons,
            Self::Wtf => DomainBackupGroup::Wtf,
            Self::Fonts => DomainBackupGroup::Fonts,
            Self::InterfaceAssets => DomainBackupGroup::InterfaceAssets,
        }
    }
}
