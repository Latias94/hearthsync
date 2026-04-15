mod storage;
#[cfg(test)]
mod tests;
mod verify;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use self::storage::{now_rfc3339, read_addon_lock};
use self::verify::{compare_lock_snapshots, lock_snapshots, snapshot_from_tracked_package};

use crate::core::addon::{
    AddonInventory, AddonPackageMetadata, AddonRegistry, AddonSourceRef, PreparedAddonPackage,
    TrackedAddon, TrackedAddonPackage, install_prepared_package, list_addons, load_registry,
    prepare_package_from_archive_with_source, prepare_package_from_source_ref_with_flavor,
    remove_selected_packages, rollback_or_report_addon_error, save_registry,
    update_prepared_packages,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

pub(crate) use self::storage::sync_addon_lock_from_registry;
pub use self::storage::{inspect_addon_lock, lock_path, write_addon_lock};
pub use self::verify::{diff_addon_locks, verify_addon_lock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonLock {
    pub schema_version: u32,
    pub generated_at: String,
    pub packages: Vec<AddonLockPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonLockPackage {
    pub package_id: String,
    #[serde(default)]
    pub index_name: Option<String>,
    #[serde(default)]
    pub index_package_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    pub source: AddonSourceRef,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    pub content_sha256: String,
    pub installed_at: String,
    pub updated_at: String,
    pub addon_directories: Vec<String>,
    pub addons: Vec<TrackedAddon>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockInspection {
    pub lock_path: PathBuf,
    pub lock: AddonLock,
    pub package_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockWriteResult {
    pub lock_path: PathBuf,
    pub package_count: usize,
    pub removed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageSnapshot {
    pub comparison_key: String,
    pub package_id: String,
    pub index_name: Option<String>,
    pub index_package_id: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub source: AddonSourceRef,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub source_sha256: Option<String>,
    pub content_sha256: Option<String>,
    pub addon_directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockFieldChange {
    pub field: String,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDiff {
    pub comparison_key: String,
    pub left: AddonLockPackageSnapshot,
    pub right: AddonLockPackageSnapshot,
    pub changes: Vec<AddonLockFieldChange>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockDiffResult {
    pub left_label: String,
    pub right_label: String,
    pub left_package_count: usize,
    pub right_package_count: usize,
    pub identical: bool,
    pub unchanged_packages: usize,
    pub added_packages: Vec<AddonLockPackageSnapshot>,
    pub removed_packages: Vec<AddonLockPackageSnapshot>,
    pub changed_packages: Vec<AddonLockPackageDiff>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockVerifyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub tracked_package_count: usize,
    pub untracked_addons: Vec<String>,
    pub missing_addon_directories: Vec<AddonLockPackageDirectoryIssue>,
    pub diff: AddonLockDiffResult,
    pub matches: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPackageDirectoryIssue {
    pub comparison_key: String,
    pub package_id: String,
    pub missing_addon_directories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AddonLockSyncActionKind {
    Install,
    Update,
    Remove,
    MetadataOnly,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockSyncAction {
    pub kind: AddonLockSyncActionKind,
    pub comparison_key: String,
    pub package_id: String,
    pub name: Option<String>,
    pub addon_directories: Vec<String>,
    pub source: Option<AddonSourceRef>,
    pub reasons: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub requires_replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPlanResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addons: Vec<String>,
    pub actions: Vec<AddonLockSyncAction>,
}

#[derive(Debug, Clone)]
pub struct AddonLockApplyRequest {
    pub installation: DetectedFlavorInstallation,
    pub lock_path: Option<PathBuf>,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub source_overrides: Vec<AddonLockSourceOverride>,
}

#[derive(Debug, Clone)]
pub struct AddonLockSourceOverride {
    pub comparison_key: String,
    pub archive_path: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
struct AddonLockSidecarSourceIndex {
    schema_version: u32,
    sources: Vec<AddonLockSidecarSourceEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct AddonLockSidecarSourceEntry {
    comparison_key: String,
    path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockApplyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addons: Vec<String>,
    pub actions: Vec<AddonLockSyncAction>,
    pub verification: AddonLockVerifyResult,
}

#[derive(Debug, Clone)]
struct PlannedLockAction {
    action: AddonLockSyncAction,
    expected: Option<AddonLockPackage>,
    current: Option<TrackedAddonPackage>,
}

#[derive(Debug, Clone)]
struct AddonLockPlanContext {
    result: AddonLockPlanResult,
    actions: Vec<PlannedLockAction>,
}

#[derive(Debug)]
struct PreparedAddonLockApply {
    remove_packages: Vec<TrackedAddonPackage>,
    update_current_packages: Vec<TrackedAddonPackage>,
    update_prepared_packages: Vec<PreparedAddonPackage>,
    install_prepared_packages: Vec<PreparedAddonPackage>,
    metadata_actions: Vec<MetadataOnlyAddonLockAction>,
}

impl PreparedAddonLockApply {
    fn is_empty(&self) -> bool {
        self.remove_packages.is_empty()
            && self.update_current_packages.is_empty()
            && self.install_prepared_packages.is_empty()
            && self.metadata_actions.is_empty()
    }
}

#[derive(Debug, Clone)]
struct MetadataOnlyAddonLockAction {
    current: TrackedAddonPackage,
    expected: AddonLockPackage,
}

pub fn plan_addon_lock_sync(
    installation: &DetectedFlavorInstallation,
    expected_lock_path: Option<&Path>,
) -> AppResult<AddonLockPlanResult> {
    plan_addon_lock_sync_with_source_overrides(installation, expected_lock_path, &[])
}

pub fn plan_addon_lock_sync_with_source_overrides(
    installation: &DetectedFlavorInstallation,
    expected_lock_path: Option<&Path>,
    source_overrides: &[AddonLockSourceOverride],
) -> AppResult<AddonLockPlanResult> {
    Ok(build_addon_lock_plan(installation, expected_lock_path, source_overrides)?.result)
}

pub fn apply_addon_lock_sync(request: AddonLockApplyRequest) -> AppResult<AddonLockApplyResult> {
    let plan = build_addon_lock_plan(
        &request.installation,
        request.lock_path.as_deref(),
        &request.source_overrides,
    )?;
    let source_overrides =
        resolved_source_override_map(&plan.result.lock_path, &request.source_overrides)?;
    let blocked_actions = plan
        .actions
        .iter()
        .filter(|action| !action.action.blocked_reasons.is_empty())
        .collect::<Vec<_>>();
    if !blocked_actions.is_empty() {
        return Err(AppError::Validation(format!(
            "cannot apply addon lock because some actions are blocked: {}",
            blocked_actions
                .iter()
                .map(|action| {
                    format!(
                        "{} ({})",
                        action.action.package_id,
                        action.action.blocked_reasons.join("; ")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let replace_required = plan
        .actions
        .iter()
        .filter(|action| {
            action.action.requires_replace_existing
                && matches!(
                    action.action.kind,
                    AddonLockSyncActionKind::Install | AddonLockSyncActionKind::Update
                )
        })
        .collect::<Vec<_>>();
    if !request.replace_existing && !replace_required.is_empty() {
        return Err(AppError::Validation(format!(
            "lock apply needs `--replace-existing` for packages: {}",
            replace_required
                .iter()
                .map(|action| action.action.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let prepared = prepare_addon_lock_apply(&plan, &source_overrides, &request.installation)?;
    let backup_path = if prepared.is_empty() {
        None
    } else {
        Some(
            create_backup(BackupRequest {
                installation: request.installation.clone(),
                output_path: request.backup_output_path.clone(),
                groups: vec![BackupGroup::Addons],
                label: Some("addon-lock-apply".to_string()),
            })?
            .archive_path,
        )
    };

    if let Err(error) =
        execute_prepared_addon_lock_apply(&request.installation, prepared, request.replace_existing)
    {
        return rollback_or_report_addon_error(
            error,
            backup_path.as_deref(),
            &request.installation,
        );
    }

    let verification = verify_addon_lock(&request.installation, Some(&plan.result.lock_path))?;
    Ok(AddonLockApplyResult {
        lock_path: plan.result.lock_path,
        installation_root: plan.result.installation_root,
        install_count: plan.result.install_count,
        update_count: plan.result.update_count,
        remove_count: plan.result.remove_count,
        metadata_only_count: plan.result.metadata_only_count,
        unchanged_count: plan.result.unchanged_count,
        blocked_count: plan.result.blocked_count,
        untracked_addons: verification.untracked_addons.clone(),
        actions: plan.result.actions,
        verification,
    })
}

fn expected_package_map(lock: &AddonLock) -> AppResult<BTreeMap<String, AddonLockPackage>> {
    let mut map = BTreeMap::new();
    for package in &lock.packages {
        let key = comparison_key(
            &package.package_id,
            package.index_name.as_deref(),
            package.index_package_id.as_deref(),
            &package.addon_directories,
        );
        if map.insert(key.clone(), package.clone()).is_some() {
            return Err(AppError::Validation(format!(
                "duplicate expected lock comparison key: {key}"
            )));
        }
    }
    Ok(map)
}

fn current_package_map(
    inventory: &AddonInventory,
) -> AppResult<BTreeMap<String, TrackedAddonPackage>> {
    let mut map = BTreeMap::new();
    for package in &inventory.tracked_packages {
        let metadata = package.metadata.as_ref();
        let key = comparison_key(
            &package.package_id,
            metadata.and_then(|value| value.index_name.as_deref()),
            metadata.and_then(|value| value.index_package_id.as_deref()),
            &package
                .addons
                .iter()
                .map(|addon| addon.directory_name.clone())
                .collect::<Vec<_>>(),
        );
        if map.insert(key.clone(), package.clone()).is_some() {
            return Err(AppError::Validation(format!(
                "duplicate current package comparison key: {key}"
            )));
        }
    }
    Ok(map)
}

fn missing_directory_map(
    issues: &[AddonLockPackageDirectoryIssue],
) -> BTreeMap<String, Vec<String>> {
    issues
        .iter()
        .map(|issue| {
            (
                issue.comparison_key.clone(),
                issue.missing_addon_directories.clone(),
            )
        })
        .collect()
}

fn tracked_directory_owner_map(inventory: &AddonInventory) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for package in &inventory.tracked_packages {
        let metadata = package.metadata.as_ref();
        let key = comparison_key(
            &package.package_id,
            metadata.and_then(|value| value.index_name.as_deref()),
            metadata.and_then(|value| value.index_package_id.as_deref()),
            &package
                .addons
                .iter()
                .map(|addon| addon.directory_name.clone())
                .collect::<Vec<_>>(),
        );
        for addon in &package.addons {
            map.insert(addon.directory_name.to_ascii_lowercase(), key.clone());
        }
    }
    map
}

fn freed_directory_set(
    current_packages: &BTreeMap<String, TrackedAddonPackage>,
    remove_keys: &BTreeSet<String>,
    update_keys: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut freed = BTreeSet::new();
    for key in remove_keys.iter().chain(update_keys.iter()) {
        if let Some(package) = current_packages.get(key) {
            for addon in &package.addons {
                freed.insert(addon.directory_name.to_ascii_lowercase());
            }
        }
    }
    freed
}

fn preflight_source_ref(source: &AddonSourceRef) -> Vec<String> {
    match source {
        AddonSourceRef::LocalArchive { path } if !path.is_file() => {
            vec![format!(
                "local archive is not available: {}",
                path.display()
            )]
        }
        _ => Vec::new(),
    }
}

fn preflight_expected_source(
    comparison_key: &str,
    source: &AddonSourceRef,
    source_overrides: &BTreeMap<String, PathBuf>,
) -> Vec<String> {
    if let Some(path) = source_overrides.get(comparison_key) {
        if path.is_file() {
            return Vec::new();
        }
        return vec![format!(
            "bundle addon source archive is not available: {}",
            path.display()
        )];
    }

    preflight_source_ref(source)
}

fn directory_conflicts(
    comparison_key: &str,
    addon_directories: &[String],
    occupied_by_tracked: &BTreeMap<String, String>,
    freed_directories: &BTreeSet<String>,
    untracked_addons: &BTreeSet<String>,
) -> (Vec<String>, bool) {
    let mut blocked_reasons = Vec::new();
    let mut requires_replace_existing = false;

    for addon_directory in addon_directories {
        let normalized = addon_directory.to_ascii_lowercase();
        if untracked_addons.contains(&normalized) {
            requires_replace_existing = true;
        }

        let Some(owner_key) = occupied_by_tracked.get(&normalized) else {
            continue;
        };
        if owner_key == comparison_key || freed_directories.contains(&normalized) {
            continue;
        }

        blocked_reasons.push(format!(
            "addon directory `{}` is owned by tracked package `{}`",
            addon_directory, owner_key
        ));
    }

    (blocked_reasons, requires_replace_existing)
}

fn has_material_changes(changes: &[AddonLockFieldChange]) -> bool {
    changes.iter().any(|change| {
        matches!(
            change.field.as_str(),
            "source" | "content_sha256" | "addon_directories"
        )
    })
}

fn metadata_from_lock_package(package: &AddonLockPackage) -> Option<AddonPackageMetadata> {
    let metadata = AddonPackageMetadata {
        index_name: package.index_name.clone(),
        index_package_id: package.index_package_id.clone(),
        package_name: package.name.clone(),
        version: package.version.clone(),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.source_sha256.clone(),
        supported_flavors: Vec::new(),
    };
    (metadata != AddonPackageMetadata::default()).then_some(metadata)
}

fn prepare_addon_lock_apply(
    plan: &AddonLockPlanContext,
    source_overrides: &BTreeMap<String, PathBuf>,
    installation: &DetectedFlavorInstallation,
) -> AppResult<PreparedAddonLockApply> {
    let mut remove_packages = Vec::new();
    let mut update_current_packages = Vec::new();
    let mut update_prepared_packages = Vec::new();
    let mut install_prepared_packages = Vec::new();
    let mut metadata_actions = Vec::new();

    for action in &plan.actions {
        match action.action.kind {
            AddonLockSyncActionKind::Remove => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock remove action is missing current package".to_string(),
                    )
                })?;
                remove_packages.push(current.clone());
            }
            AddonLockSyncActionKind::Update => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock update action is missing current package".to_string(),
                    )
                })?;
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock update action is missing expected package".to_string(),
                    )
                })?;
                let mut prepared = prepare_expected_lock_package(
                    expected,
                    source_overrides
                        .get(&action.action.comparison_key)
                        .map(PathBuf::as_path),
                    installation.flavor,
                )?;
                prepared.metadata = metadata_from_lock_package(expected);
                update_current_packages.push(current.clone());
                update_prepared_packages.push(prepared);
            }
            AddonLockSyncActionKind::Install => {
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock install action is missing expected package".to_string(),
                    )
                })?;
                let mut prepared = prepare_expected_lock_package(
                    expected,
                    source_overrides
                        .get(&action.action.comparison_key)
                        .map(PathBuf::as_path),
                    installation.flavor,
                )?;
                prepared.metadata = metadata_from_lock_package(expected);
                install_prepared_packages.push(prepared);
            }
            AddonLockSyncActionKind::MetadataOnly => {
                let current = action.current.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock metadata-only action is missing current package".to_string(),
                    )
                })?;
                let expected = action.expected.as_ref().ok_or_else(|| {
                    AppError::Validation(
                        "lock metadata-only action is missing expected package".to_string(),
                    )
                })?;
                metadata_actions.push(MetadataOnlyAddonLockAction {
                    current: current.clone(),
                    expected: expected.clone(),
                });
            }
        }
    }

    Ok(PreparedAddonLockApply {
        remove_packages,
        update_current_packages,
        update_prepared_packages,
        install_prepared_packages,
        metadata_actions,
    })
}

fn execute_prepared_addon_lock_apply(
    installation: &DetectedFlavorInstallation,
    prepared: PreparedAddonLockApply,
    replace_existing: bool,
) -> AppResult<()> {
    if !prepared.remove_packages.is_empty() {
        remove_selected_packages(installation, prepared.remove_packages)?;
    }

    if !prepared.update_current_packages.is_empty() {
        let registry = load_registry(installation)?;
        update_prepared_packages(
            installation,
            registry,
            prepared.update_current_packages,
            prepared.update_prepared_packages,
        )?;
    }

    for prepared_package in prepared.install_prepared_packages {
        install_prepared_package(installation, prepared_package, replace_existing)?;
    }

    if !prepared.metadata_actions.is_empty() {
        apply_metadata_only_actions(installation, prepared.metadata_actions)?;
    }

    Ok(())
}

fn apply_metadata_only_actions(
    installation: &DetectedFlavorInstallation,
    actions: Vec<MetadataOnlyAddonLockAction>,
) -> AppResult<()> {
    let mut registry = load_registry(installation)?;
    let timestamp = now_rfc3339()?;

    for action in actions {
        let package = registry
            .packages
            .iter_mut()
            .find(|candidate| **candidate == action.current)
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "tracked package disappeared before metadata apply: {}",
                    action.current.package_id
                ))
            })?;
        package.package_id = action.expected.package_id.clone();
        package.updated_at = timestamp.clone();
        package.metadata = metadata_from_lock_package(&action.expected);
    }

    save_registry(installation, &registry)
}

fn lock_action_sort_key(kind: &AddonLockSyncActionKind) -> u8 {
    match kind {
        AddonLockSyncActionKind::Remove => 0,
        AddonLockSyncActionKind::Update => 1,
        AddonLockSyncActionKind::Install => 2,
        AddonLockSyncActionKind::MetadataOnly => 3,
    }
}

fn build_addon_lock_plan(
    installation: &DetectedFlavorInstallation,
    expected_lock_path: Option<&Path>,
    source_overrides: &[AddonLockSourceOverride],
) -> AppResult<AddonLockPlanContext> {
    let lock_path = expected_lock_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| lock_path(installation));
    let source_overrides = resolved_source_override_map(&lock_path, source_overrides)?;
    let expected_lock = read_addon_lock(&lock_path)?;
    let inventory = list_addons(installation)?;
    let current_snapshots = inventory
        .tracked_packages
        .iter()
        .map(|package| snapshot_from_tracked_package(installation, package))
        .collect::<Vec<_>>();
    let missing_addon_directories = current_snapshots
        .iter()
        .filter_map(|(snapshot, missing)| {
            (!missing.is_empty()).then_some(AddonLockPackageDirectoryIssue {
                comparison_key: snapshot.comparison_key.clone(),
                package_id: snapshot.package_id.clone(),
                missing_addon_directories: missing.clone(),
            })
        })
        .collect::<Vec<_>>();
    let current_snapshot_values = current_snapshots
        .iter()
        .map(|(snapshot, _)| snapshot.clone())
        .collect::<Vec<_>>();
    let diff = compare_lock_snapshots(
        &lock_path.display().to_string(),
        &lock_snapshots(&expected_lock)?,
        &installation.flavor_root.display().to_string(),
        &current_snapshot_values,
    )?;

    let expected_packages = expected_package_map(&expected_lock)?;
    let current_packages = current_package_map(&inventory)?;
    let remove_keys = diff
        .added_packages
        .iter()
        .map(|package| package.comparison_key.clone())
        .collect::<BTreeSet<_>>();
    let mut update_keys = BTreeSet::new();
    let missing_map = missing_directory_map(&missing_addon_directories);
    for package in &diff.changed_packages {
        let has_missing = missing_map.contains_key(&package.comparison_key);
        if has_missing || has_material_changes(&package.changes) {
            update_keys.insert(package.comparison_key.clone());
        }
    }

    let occupied_by_tracked = tracked_directory_owner_map(&inventory);
    let freed_directories = freed_directory_set(&current_packages, &remove_keys, &update_keys);
    let untracked_addons = inventory
        .untracked_addons
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    let mut actions = Vec::new();
    for package in &diff.removed_packages {
        let expected = expected_packages
            .get(&package.comparison_key)
            .cloned()
            .ok_or_else(|| AppError::Validation("expected lock package missing".to_string()))?;
        let blocked_reasons =
            preflight_expected_source(&package.comparison_key, &expected.source, &source_overrides);
        let (conflict_reasons, requires_replace_existing) = directory_conflicts(
            &package.comparison_key,
            &package.addon_directories,
            &occupied_by_tracked,
            &freed_directories,
            &untracked_addons,
        );
        actions.push(PlannedLockAction {
            action: AddonLockSyncAction {
                kind: AddonLockSyncActionKind::Install,
                comparison_key: package.comparison_key.clone(),
                package_id: expected.package_id.clone(),
                name: expected.name.clone(),
                addon_directories: expected.addon_directories.clone(),
                source: Some(expected.source.clone()),
                reasons: vec!["package is missing from the current installation".to_string()],
                blocked_reasons: blocked_reasons
                    .into_iter()
                    .chain(conflict_reasons.into_iter())
                    .collect(),
                requires_replace_existing,
            },
            expected: Some(expected),
            current: None,
        });
    }

    for package in &diff.added_packages {
        let current = current_packages
            .get(&package.comparison_key)
            .cloned()
            .ok_or_else(|| AppError::Validation("current tracked package missing".to_string()))?;
        actions.push(PlannedLockAction {
            action: AddonLockSyncAction {
                kind: AddonLockSyncActionKind::Remove,
                comparison_key: package.comparison_key.clone(),
                package_id: current.package_id.clone(),
                name: package.name.clone(),
                addon_directories: package.addon_directories.clone(),
                source: Some(current.source.clone()),
                reasons: vec!["package is not present in the expected lock".to_string()],
                blocked_reasons: Vec::new(),
                requires_replace_existing: false,
            },
            expected: None,
            current: Some(current),
        });
    }

    for package in &diff.changed_packages {
        let expected = expected_packages
            .get(&package.comparison_key)
            .cloned()
            .ok_or_else(|| AppError::Validation("expected lock package missing".to_string()))?;
        let current = current_packages
            .get(&package.comparison_key)
            .cloned()
            .ok_or_else(|| AppError::Validation("current tracked package missing".to_string()))?;
        let has_missing = missing_map
            .get(&package.comparison_key)
            .map(|item| !item.is_empty())
            .unwrap_or(false);
        let physical_change = has_missing || has_material_changes(&package.changes);
        let kind = if physical_change {
            AddonLockSyncActionKind::Update
        } else {
            AddonLockSyncActionKind::MetadataOnly
        };
        let mut reasons = package
            .changes
            .iter()
            .map(|change| format!("{} differs", change.field))
            .collect::<Vec<_>>();
        if let Some(missing) = missing_map.get(&package.comparison_key) {
            if !missing.is_empty() {
                reasons.push(format!(
                    "tracked addon directories are missing: {}",
                    missing.join(", ")
                ));
            }
        }
        let blocked_reasons = if kind == AddonLockSyncActionKind::Update {
            preflight_expected_source(&package.comparison_key, &expected.source, &source_overrides)
        } else {
            Vec::new()
        };
        let (conflict_reasons, requires_replace_existing) =
            if kind == AddonLockSyncActionKind::Update {
                directory_conflicts(
                    &package.comparison_key,
                    &expected.addon_directories,
                    &occupied_by_tracked,
                    &freed_directories,
                    &untracked_addons,
                )
            } else {
                (Vec::new(), false)
            };
        actions.push(PlannedLockAction {
            action: AddonLockSyncAction {
                kind,
                comparison_key: package.comparison_key.clone(),
                package_id: expected.package_id.clone(),
                name: expected.name.clone(),
                addon_directories: expected.addon_directories.clone(),
                source: Some(expected.source.clone()),
                reasons,
                blocked_reasons: blocked_reasons
                    .into_iter()
                    .chain(conflict_reasons.into_iter())
                    .collect(),
                requires_replace_existing,
            },
            expected: Some(expected),
            current: Some(current),
        });
    }

    actions.sort_by(|left, right| {
        lock_action_sort_key(&left.action.kind)
            .cmp(&lock_action_sort_key(&right.action.kind))
            .then_with(|| left.action.comparison_key.cmp(&right.action.comparison_key))
    });

    let install_count = actions
        .iter()
        .filter(|action| action.action.kind == AddonLockSyncActionKind::Install)
        .count();
    let update_count = actions
        .iter()
        .filter(|action| action.action.kind == AddonLockSyncActionKind::Update)
        .count();
    let remove_count = actions
        .iter()
        .filter(|action| action.action.kind == AddonLockSyncActionKind::Remove)
        .count();
    let metadata_only_count = actions
        .iter()
        .filter(|action| action.action.kind == AddonLockSyncActionKind::MetadataOnly)
        .count();
    let blocked_count = actions
        .iter()
        .filter(|action| !action.action.blocked_reasons.is_empty())
        .count();

    Ok(AddonLockPlanContext {
        result: AddonLockPlanResult {
            lock_path,
            installation_root: installation.flavor_root.clone(),
            install_count,
            update_count,
            remove_count,
            metadata_only_count,
            unchanged_count: diff.unchanged_packages,
            blocked_count,
            untracked_addons: inventory.untracked_addons,
            actions: actions.iter().map(|action| action.action.clone()).collect(),
        },
        actions,
    })
}

fn resolved_source_override_map(
    lock_path: &Path,
    source_overrides: &[AddonLockSourceOverride],
) -> AppResult<BTreeMap<String, PathBuf>> {
    let mut map = load_sidecar_source_overrides(lock_path)?;
    let mut explicit_keys = BTreeSet::new();
    for source_override in source_overrides {
        if source_override.comparison_key.trim().is_empty() {
            return Err(AppError::Validation(
                "addon lock source override comparison key must not be empty".to_string(),
            ));
        }
        if !explicit_keys.insert(source_override.comparison_key.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate addon lock source override for `{}`",
                source_override.comparison_key
            )));
        }
        map.insert(
            source_override.comparison_key.clone(),
            source_override.archive_path.clone(),
        );
    }
    Ok(map)
}

fn load_sidecar_source_overrides(lock_path: &Path) -> AppResult<BTreeMap<String, PathBuf>> {
    let Some(lock_dir) = lock_path.parent() else {
        return Ok(BTreeMap::new());
    };
    let source_index_path = lock_dir.join("sources.toml");
    if !source_index_path.is_file() {
        return Ok(BTreeMap::new());
    }

    let content = fs::read_to_string(&source_index_path)?;
    let source_index = toml::from_str::<AddonLockSidecarSourceIndex>(&content)?;
    if source_index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon lock source index schema version: {}",
            source_index.schema_version
        )));
    }

    let mut map = BTreeMap::new();
    for source in source_index.sources {
        if source.comparison_key.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "addon lock source index `{}` contains an empty comparison key",
                source_index_path.display()
            )));
        }
        let segments = safe_sidecar_source_segments(&source.path)?;
        let archive_path = join_sidecar_source_segments(lock_dir, &segments);
        if map
            .insert(source.comparison_key.clone(), archive_path)
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "duplicate addon lock source index entry for `{}`",
                source.comparison_key
            )));
        }
    }

    Ok(map)
}

fn safe_sidecar_source_segments(path: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in path.split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe addon lock source path: `{path}`"
            )));
        }
        segments.push(segment);
    }
    if segments.first().copied() != Some("sources") || segments.len() < 2 {
        return Err(AppError::Validation(format!(
            "addon lock source path must be under `sources/`: {path}"
        )));
    }
    Ok(segments)
}

fn join_sidecar_source_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

fn prepare_expected_lock_package(
    expected: &AddonLockPackage,
    source_override_path: Option<&Path>,
    target_flavor: crate::core::install::WowFlavor,
) -> AppResult<crate::core::addon::PreparedAddonPackage> {
    match source_override_path {
        Some(path) => prepare_package_from_archive_with_source(expected.source.clone(), path),
        None => prepare_package_from_source_ref_with_flavor(&expected.source, Some(target_flavor)),
    }
}

fn comparison_key(
    package_id: &str,
    index_name: Option<&str>,
    index_package_id: Option<&str>,
    addon_directories: &[String],
) -> String {
    let index_name = index_name.map(str::trim).filter(|value| !value.is_empty());
    let index_package_id = index_package_id
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (index_name, index_package_id) {
        (Some(index_name), Some(index_package_id)) => {
            format!("index:{index_name}:{index_package_id}")
        }
        (None, Some(index_package_id)) => format!("index:{index_package_id}"),
        _ => {
            let mut normalized = addon_directories
                .iter()
                .map(|item| item.trim().to_ascii_lowercase())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>();
            normalized.sort();
            normalized.dedup();
            if normalized.is_empty() {
                format!("package:{package_id}")
            } else {
                format!("addons:{}", normalized.join("+"))
            }
        }
    }
}

pub(crate) fn addon_lock_package_comparison_key(package: &AddonLockPackage) -> String {
    comparison_key(
        &package.package_id,
        package.index_name.as_deref(),
        package.index_package_id.as_deref(),
        &package.addon_directories,
    )
}

fn left_label(path: &Path) -> String {
    path.display().to_string()
}
