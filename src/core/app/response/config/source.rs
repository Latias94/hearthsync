use serde::Serialize;

use super::super::external_package::ExternalPackageSourceKindResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigPackageSourceKindResult {
    Directory,
    ZipArchive,
}

impl ConfigPackageSourceKindResult {
    pub(super) fn from_external(value: ExternalPackageSourceKindResult) -> Self {
        match value {
            ExternalPackageSourceKindResult::Directory => Self::Directory,
            ExternalPackageSourceKindResult::ZipArchive => Self::ZipArchive,
        }
    }
}
