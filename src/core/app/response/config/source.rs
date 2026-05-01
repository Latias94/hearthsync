use serde::Serialize;

use crate::core::bundle::ExternalPackageSourceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPackageSourceKindResult {
    Directory,
    ZipArchive,
}

impl ConfigPackageSourceKindResult {
    pub(super) fn from_external(value: ExternalPackageSourceKind) -> Self {
        match value {
            ExternalPackageSourceKind::Directory => Self::Directory,
            ExternalPackageSourceKind::ZipArchive => Self::ZipArchive,
        }
    }
}
