use std::path::PathBuf;

use crate::core::lua_patch::CharacterMapping;

use super::{ApplyAction, ApplyGroup, ApplyOperation, BundleApplyPlan, WtfScope};

#[derive(Debug, Clone)]
pub(super) struct PlannedEntry {
    pub(super) archive_name: String,
    pub(super) destination: PathBuf,
    pub(super) rewrites: Vec<CharacterMapping>,
    pub(super) group: ApplyGroup,
    pub(super) wtf_scope: Option<WtfScope>,
    pub(super) target_account: Option<String>,
    pub(super) target_server: Option<String>,
    pub(super) target_character: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedCleanup {
    pub(super) group: ApplyGroup,
    pub(super) destination: PathBuf,
    pub(super) target_account: Option<String>,
    pub(super) target_server: Option<String>,
    pub(super) target_character: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedApplyOperation {
    pub(super) group: ApplyGroup,
    pub(super) wtf_scope: Option<WtfScope>,
    pub(super) action: ApplyAction,
    pub(super) archive_name: String,
    pub(super) destination: PathBuf,
    pub(super) target_account: Option<String>,
    pub(super) target_server: Option<String>,
    pub(super) target_character: Option<String>,
    pub(super) rewrite_applied: bool,
    pub(super) rewrites: Vec<CharacterMapping>,
}

impl PreparedApplyOperation {
    pub(super) fn from_cleanup(cleanup: PlannedCleanup) -> Self {
        Self {
            group: cleanup.group,
            wtf_scope: None,
            action: ApplyAction::Remove,
            archive_name: format!("[cleanup] {}", cleanup.destination.display()),
            destination: cleanup.destination,
            target_account: cleanup.target_account,
            target_server: cleanup.target_server,
            target_character: cleanup.target_character,
            rewrite_applied: false,
            rewrites: Vec::new(),
        }
    }

    pub(super) fn from_entry(
        entry: &PlannedEntry,
        action: ApplyAction,
        rewrite_applied: bool,
    ) -> Self {
        Self {
            group: entry.group,
            wtf_scope: entry.wtf_scope,
            action,
            archive_name: entry.archive_name.clone(),
            destination: entry.destination.clone(),
            target_account: entry.target_account.clone(),
            target_server: entry.target_server.clone(),
            target_character: entry.target_character.clone(),
            rewrite_applied,
            rewrites: entry.rewrites.clone(),
        }
    }

    pub(super) fn preview(&self) -> ApplyOperation {
        ApplyOperation {
            group: self.group,
            wtf_scope: self.wtf_scope,
            action: self.action,
            archive_name: self.archive_name.clone(),
            destination: self.destination.clone(),
            target_account: self.target_account.clone(),
            target_server: self.target_server.clone(),
            target_character: self.target_character.clone(),
            rewrite_count: self.rewrites.len(),
            rewrite_applied: self.rewrite_applied,
        }
    }
}

pub(super) struct PreparedBundleApply {
    pub(super) plan: BundleApplyPlan,
    pub(super) execution_operations: Vec<PreparedApplyOperation>,
}
