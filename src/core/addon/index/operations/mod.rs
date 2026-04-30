use std::collections::BTreeSet;
use std::path::PathBuf;

use super::matching::{
    explain_preflight_match_index_package_to_tracked_package,
    match_index_package_to_tracked_package, package_id_usage_key,
    preflight_match_index_package_to_tracked_package,
};
use super::storage::{
    ensure_package_supports_flavor, find_index_package, load_addon_index,
    resolve_index_package_source,
};
use crate::core::addon::{
    AddonPackageMetadata, AddonProvider, AddonRegistry, AddonSourceRef, AddonStatePaths,
    DefaultAddonProvider, InstallAddonExecutionPlan, InstallPreparedAddonRequest,
    MissingDependencyCollectionRequest, MissingDependencyCollectionState,
    PreparePackageFromSourceRefTaskRequest, PreparePackageTaskContext, PreparedAddonPackage,
    TrackedAddonPackage, UpdatePreparedPackagesWithDependenciesRequest, UpdatedAddonPackageResult,
    collect_missing_dependency_prepared_packages, ensure_relink_addon_directories_match,
    execute_install_plan_task, list_addons, load_registry, no_tracked_packages_error,
    policy::AddonUpdatePolicySnapshot, prepare_install_prepared_addon,
    prepare_package_from_source_ref_task_with_provider, preview_installed_dependency_packages,
    provider::AddonSourceResolutionPolicy, relink_source_changed, relink_timestamp,
    rollback_or_report_addon_error, save_registry, select_single_tracked_package,
    update_prepared_packages_with_dependencies_task, validate_dependency_resolution_support,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, NeverCancel, NoopProgressSink, TaskKind, TaskPhase, TaskProgressEvent,
    TaskProgressSink, emit_task_progress, ensure_task_not_cancelled,
};

use super::{
    AddonIndex, AddonIndexAttachPackageResult, AddonIndexAttachPackageStatus,
    AddonIndexAttachRequest, AddonIndexAttachResult, AddonIndexInstallRequest,
    AddonIndexInstallResult, AddonIndexPackage, AddonIndexRelinkRequest, AddonIndexRelinkResult,
    AddonIndexTrackedMatchStrategy, AddonIndexUpdateRequest, AddonIndexUpdateResult,
};

struct IndexInstallPlan {
    index_path: PathBuf,
    package: AddonIndexPackage,
    install_plan: InstallAddonExecutionPlan,
}

struct IndexAttachPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    index_path: PathBuf,
    index_name: String,
    dry_run: bool,
    apply_ready_only: bool,
    registry_path: PathBuf,
    index_package_count: usize,
    considered_package_count: usize,
    skipped_unsupported_flavor_package_count: usize,
    packages: Vec<AddonIndexAttachPackageResult>,
    changes: Vec<IndexAttachChange>,
}

struct IndexAttachChange {
    package_result_index: usize,
    package: AddonIndexPackage,
    tracked_package_index: usize,
    tracked_package: TrackedAddonPackage,
    next_source: AddonSourceRef,
    metadata: AddonPackageMetadata,
    match_strategy: AddonIndexTrackedMatchStrategy,
    source_changed: bool,
    metadata_changed: bool,
}

struct IndexUpdatePlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    index_path: PathBuf,
    selected_packages: Vec<AddonIndexPackage>,
    registry: AddonRegistry,
    prepared_packages: Vec<PreparedAddonPackage>,
    dependency_prepared_packages: Vec<PreparedAddonPackage>,
    matched_packages: Vec<TrackedAddonPackage>,
    ignored_packages: Vec<String>,
    dry_run: bool,
    backup_output_path: Option<PathBuf>,
    registry_path: PathBuf,
    files_to_write: usize,
}

struct IndexRelinkPlan {
    installation: DetectedFlavorInstallation,
    state_paths: AddonStatePaths,
    index_path: PathBuf,
    index_name: String,
    package: AddonIndexPackage,
    tracked_package_index: usize,
    tracked_package: TrackedAddonPackage,
    next_source: AddonSourceRef,
    metadata: AddonPackageMetadata,
    dry_run: bool,
    registry_path: PathBuf,
    source_changed: bool,
    metadata_changed: bool,
}

mod attach;
mod install;
mod relink;
mod shared;
mod update;

pub(crate) use self::attach::attach_addons_from_index_task_with_provider;
pub use self::attach::{attach_addons_from_index, attach_addons_from_index_task};
pub(crate) use self::install::install_addon_from_index_task_with_provider;
pub use self::install::{install_addon_from_index, install_addon_from_index_task};
pub(crate) use self::relink::relink_addon_from_index_task_with_provider;
pub use self::relink::{relink_addon_from_index, relink_addon_from_index_task};
use self::shared::{
    RemappedTaskProgressSink, metadata_from_index_package, remap_cancelled_task_kind,
    resolved_index_package_for_matching,
};
pub use self::update::{update_addons_from_index, update_addons_from_index_task};
pub(crate) use self::update::{
    update_addons_from_index_task_with_provider,
    validate_addon_index_update_dependency_policy_support,
};
