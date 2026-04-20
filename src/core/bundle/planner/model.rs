use std::path::PathBuf;

use super::super::{
    BundleApplyPlan, BundleManifest, CharacterMapping, LocalWowAccount, PlannedCleanup,
    PlannedEntry, PreviewOperation,
};

#[derive(Debug)]
pub(super) struct LogicalBundleApply {
    pub(super) plan_path: PathBuf,
    pub(super) target_flavor_root: PathBuf,
    pub(super) discovered_accounts: Vec<LocalWowAccount>,
    pub(super) selected_target_accounts: Vec<String>,
    pub(super) character_mappings: Vec<CharacterMapping>,
    pub(super) manifest: BundleManifest,
    pub(super) cleanup_operations: Vec<PlannedCleanup>,
    pub(super) entry_operations: Vec<LogicalEntryOperation>,
}

#[derive(Debug)]
pub(super) struct LogicalEntryOperation {
    pub(super) entry: PlannedEntry,
    pub(super) disposition: LogicalEntryDisposition,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum LogicalEntryDisposition {
    Preserve,
    Materialize { will_cleanup: bool },
}

#[derive(Debug)]
pub(super) struct PendingPreviewApply {
    pub(super) plan_path: PathBuf,
    pub(super) target_flavor_root: PathBuf,
    pub(super) discovered_accounts: Vec<LocalWowAccount>,
    pub(super) selected_target_accounts: Vec<String>,
    pub(super) character_mappings: Vec<CharacterMapping>,
    pub(super) manifest: BundleManifest,
    pub(super) settled_operations: Vec<PreviewOperation>,
    pub(super) pending_existing_target_entries: Vec<PendingExistingTargetPreviewEntry>,
}

#[derive(Debug)]
pub(super) struct PendingExistingTargetPreviewEntry {
    pub(super) entry: PlannedEntry,
}

#[derive(Debug)]
pub(super) struct ResolvedPreviewApply {
    pub(super) plan: BundleApplyPlan,
    pub(super) preview_operations: Vec<PreviewOperation>,
}
