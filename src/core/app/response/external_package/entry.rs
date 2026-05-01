use serde::Serialize;

use crate::core::app::{ApplyGroupValue, WtfScopeValue};
use crate::core::bundle::ExternalPackageEntry as DomainExternalPackageEntry;

#[derive(Debug, Clone, Serialize)]
pub struct ExternalPackageEntryResult {
    pub source_path: String,
    pub normalized_path: String,
    pub group: ApplyGroupValue,
    pub wtf_scope: Option<WtfScopeValue>,
    pub source_account: Option<String>,
    pub source_server: Option<String>,
    pub source_character: Option<String>,
}

impl ExternalPackageEntryResult {
    pub(crate) fn from_domain(value: DomainExternalPackageEntry) -> Self {
        Self {
            source_path: value.source_path,
            normalized_path: value.normalized_path,
            group: ApplyGroupValue::from_domain(value.group),
            wtf_scope: value.wtf_scope.map(WtfScopeValue::from_domain),
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
        }
    }
}
