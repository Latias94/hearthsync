use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    BackupRestore,
    BundleApply,
    AddonLockApply,
    AddonInstall,
    AddonUpdate,
    AddonRemove,
    AddonIndexAttach,
    AddonIndexInstall,
    AddonIndexUpdate,
    AddonIndexRelink,
    ExternalPackageAnalyze,
    ExternalPackagePlan,
    ExternalPackageApply,
}

impl TaskKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BackupRestore => "backup_restore",
            Self::BundleApply => "bundle_apply",
            Self::AddonLockApply => "addon_lock_apply",
            Self::AddonInstall => "addon_install",
            Self::AddonUpdate => "addon_update",
            Self::AddonRemove => "addon_remove",
            Self::AddonIndexAttach => "addon_index_attach",
            Self::AddonIndexInstall => "addon_index_install",
            Self::AddonIndexUpdate => "addon_index_update",
            Self::AddonIndexRelink => "addon_index_relink",
            Self::ExternalPackageAnalyze => "external_package_analyze",
            Self::ExternalPackagePlan => "external_package_plan",
            Self::ExternalPackageApply => "external_package_apply",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPhase {
    Preparing,
    Planning,
    BackingUp,
    Executing,
    Verifying,
    Completed,
}

impl TaskPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Planning => "planning",
            Self::BackingUp => "backing_up",
            Self::Executing => "executing",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskProgressCode {
    Preparing,
    Planning,
    BackingUp,
    Executing,
    Verifying,
    Completed,
    DownloadArchive,
    RemoveAddonDirectory,
    WriteAddonDirectory,
    ApplyMetadata,
    ClearRestoreGroup,
    RestoreEntry,
    ApplyOperation,
}

impl TaskProgressCode {
    pub fn for_phase(phase: TaskPhase) -> Self {
        match phase {
            TaskPhase::Preparing => Self::Preparing,
            TaskPhase::Planning => Self::Planning,
            TaskPhase::BackingUp => Self::BackingUp,
            TaskPhase::Executing => Self::Executing,
            TaskPhase::Verifying => Self::Verifying,
            TaskPhase::Completed => Self::Completed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskProgressEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub task: TaskKind,
    pub phase: TaskPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<TaskProgressCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_current: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_per_second: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskByteProgress {
    pub code: TaskProgressCode,
    pub bytes_current: u64,
    pub bytes_total: Option<u64>,
    pub bytes_per_second: Option<u64>,
}
