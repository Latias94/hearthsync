use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::core::lua_patch::CharacterMapping;

use super::{
    ApplyAction, ApplyGroup, ApplyOperation, BundleApplyPlan, ExternalPackageSourceKind, WtfScope,
};

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
    pub(super) action: ApplyAction,
    pub(super) archive_name: String,
    pub(super) destination: PathBuf,
    pub(super) rewrites: Vec<CharacterMapping>,
}

#[derive(Debug, Clone)]
pub(super) struct PreviewOperation {
    operation: ApplyOperation,
    rewrites: Vec<CharacterMapping>,
}

#[derive(Debug, Clone)]
pub(super) enum PreparedApplySource {
    BundleArchive {
        bundle_path: PathBuf,
    },
    ExternalPackage {
        source_path: PathBuf,
        source_kind: ExternalPackageSourceKind,
        entry_source_map: BTreeMap<String, String>,
    },
}

impl PreparedApplyOperation {
    pub(super) fn from_preview(preview_operation: PreviewOperation) -> Self {
        let PreviewOperation {
            operation:
                ApplyOperation {
                    action,
                    archive_name,
                    destination,
                    ..
                },
            rewrites,
        } = preview_operation;

        Self {
            action,
            archive_name,
            destination,
            rewrites,
        }
    }
}

impl PreviewOperation {
    pub(super) fn from_cleanup(cleanup: PlannedCleanup) -> Self {
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

    pub(super) fn from_entry(entry: &PlannedEntry, action: ApplyAction) -> Self {
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

    pub(super) fn action(&self) -> ApplyAction {
        self.operation.action
    }

    pub(super) fn group(&self) -> ApplyGroup {
        self.operation.group
    }

    pub(super) fn destination(&self) -> &Path {
        &self.operation.destination
    }

    pub(super) fn archive_name(&self) -> &str {
        &self.operation.archive_name
    }
}

impl From<PreviewOperation> for ApplyOperation {
    fn from(value: PreviewOperation) -> Self {
        value.operation
    }
}

#[derive(Debug)]
pub(super) struct PreparedBundleApply {
    pub(super) source: PreparedApplySource,
    pub(super) plan: BundleApplyPlan,
    pub(super) execution_operations: Vec<PreparedApplyOperation>,
}
