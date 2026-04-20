use std::path::Path;

use crate::core::lua_patch::CharacterMapping;

use super::super::{ApplyAction, ApplyGroup, ApplyOperation};
use super::planned::{PlannedCleanup, PlannedEntry};

#[derive(Debug, Clone)]
pub(in crate::core::bundle) struct PreviewOperation {
    operation: ApplyOperation,
    rewrites: Vec<CharacterMapping>,
}

impl PreviewOperation {
    pub(in crate::core::bundle) fn from_cleanup(cleanup: PlannedCleanup) -> Self {
        Self {
            operation: ApplyOperation {
                group: cleanup.group,
                wtf_scope: None,
                action: ApplyAction::Remove,
                archive_name: format!("[cleanup] {}", cleanup.destination.display()),
                destination: cleanup.destination,
                target_account: cleanup.target_account,
                target_server: cleanup.target_server,
                target_character: cleanup.target_character,
            },
            rewrites: Vec::new(),
        }
    }

    pub(in crate::core::bundle) fn from_entry(entry: &PlannedEntry, action: ApplyAction) -> Self {
        Self {
            operation: ApplyOperation {
                group: entry.group,
                wtf_scope: entry.wtf_scope,
                action,
                archive_name: entry.archive_name.clone(),
                destination: entry.destination.clone(),
                target_account: entry.target_account.clone(),
                target_server: entry.target_server.clone(),
                target_character: entry.target_character.clone(),
            },
            rewrites: entry.rewrites.clone(),
        }
    }

    pub(in crate::core::bundle) fn action(&self) -> ApplyAction {
        self.operation.action
    }

    pub(in crate::core::bundle) fn group(&self) -> ApplyGroup {
        self.operation.group
    }

    pub(in crate::core::bundle) fn destination(&self) -> &Path {
        &self.operation.destination
    }

    pub(in crate::core::bundle) fn archive_name(&self) -> &str {
        &self.operation.archive_name
    }

    pub(in crate::core::bundle) fn into_parts(self) -> (ApplyOperation, Vec<CharacterMapping>) {
        (self.operation, self.rewrites)
    }
}

impl From<PreviewOperation> for ApplyOperation {
    fn from(value: PreviewOperation) -> Self {
        value.operation
    }
}
