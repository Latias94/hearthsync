use serde::Serialize;

use crate::core::app::{ApplyGroupValue, WtfScopeValue};

use super::super::external_package::ExternalPackageEntryResult;

#[derive(Debug, Clone, Serialize)]
pub struct ConfigPackageEntryResult {
    pub source_path: String,
    pub normalized_path: String,
    pub group: ApplyGroupValue,
    pub wtf_scope: Option<WtfScopeValue>,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

impl ConfigPackageEntryResult {
    pub(super) fn from_external(value: ExternalPackageEntryResult) -> Self {
        Self {
            source_path: value.source_path,
            normalized_path: value.normalized_path,
            group: value.group,
            wtf_scope: value.wtf_scope,
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
        }
    }
}
