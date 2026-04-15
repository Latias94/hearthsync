use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use crate::core::addon::{
    AddonInventory, AddonPackageMetadata, AddonRegistry, AddonSourceRef,
    InstallPreparedAddonRequest, RemoveAddonRequest, TrackedAddon, TrackedAddonPackage,
    install_prepared_addon, list_addons, load_registry, prepare_package_from_archive_with_source,
    prepare_package_from_source_ref_with_flavor, remove_addons, rollback_or_report_addon_error,
    update_prepared_packages,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

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

pub fn inspect_addon_lock(
    installation: &DetectedFlavorInstallation,
) -> AppResult<AddonLockInspection> {
    let path = lock_path(installation);
    let lock = read_addon_lock(&path)?;
    let package_count = lock.packages.len();

    Ok(AddonLockInspection {
        lock_path: path,
        lock,
        package_count,
    })
}

pub fn diff_addon_locks(left_path: &Path, right_path: &Path) -> AppResult<AddonLockDiffResult> {
    let left = read_addon_lock(left_path)?;
    let right = read_addon_lock(right_path)?;
    compare_lock_snapshots(
        &left_label(left_path),
        &lock_snapshots(&left)?,
        &left_label(right_path),
        &lock_snapshots(&right)?,
    )
}

pub fn verify_addon_lock(
    installation: &DetectedFlavorInstallation,
    expected_lock_path: Option<&Path>,
) -> AppResult<AddonLockVerifyResult> {
    let lock_path = expected_lock_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| lock_path(installation));
    let expected = read_addon_lock(&lock_path)?;
    let inventory = crate::core::addon::list_addons(installation)?;

    let mut current_snapshots = Vec::new();
    let mut missing_addon_directories = Vec::new();
    for package in &inventory.tracked_packages {
        let (snapshot, missing) = snapshot_from_tracked_package(installation, package);
        if !missing.is_empty() {
            missing_addon_directories.push(AddonLockPackageDirectoryIssue {
                comparison_key: snapshot.comparison_key.clone(),
                package_id: snapshot.package_id.clone(),
                missing_addon_directories: missing,
            });
        }
        current_snapshots.push(snapshot);
    }

    let diff = compare_lock_snapshots(
        &lock_path.display().to_string(),
        &lock_snapshots(&expected)?,
        &installation.flavor_root.display().to_string(),
        &current_snapshots,
    )?;
    let matches = diff.identical
        && inventory.untracked_addons.is_empty()
        && missing_addon_directories.is_empty();

    Ok(AddonLockVerifyResult {
        lock_path,
        installation_root: installation.flavor_root.clone(),
        tracked_package_count: inventory.tracked_packages.len(),
        untracked_addons: inventory.untracked_addons,
        missing_addon_directories,
        diff,
        matches,
    })
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

    for action in plan
        .actions
        .iter()
        .filter(|action| action.action.kind == AddonLockSyncActionKind::Remove)
    {
        let current = action.current.as_ref().ok_or_else(|| {
            AppError::Validation("lock remove action is missing current package".to_string())
        })?;
        remove_addons(RemoveAddonRequest {
            installation: request.installation.clone(),
            name: current.package_id.clone(),
            dry_run: false,
            backup_output_path: request.backup_output_path.clone(),
        })?;
    }

    let update_actions = plan
        .actions
        .iter()
        .filter(|action| action.action.kind == AddonLockSyncActionKind::Update)
        .collect::<Vec<_>>();
    if !update_actions.is_empty() {
        let mut selected_packages = Vec::new();
        let mut prepared_packages = Vec::new();
        for action in &update_actions {
            let current = action.current.as_ref().ok_or_else(|| {
                AppError::Validation("lock update action is missing current package".to_string())
            })?;
            let expected = action.expected.as_ref().ok_or_else(|| {
                AppError::Validation("lock update action is missing expected package".to_string())
            })?;
            let mut prepared = prepare_expected_lock_package(
                expected,
                source_overrides
                    .get(&action.action.comparison_key)
                    .map(PathBuf::as_path),
                request.installation.flavor,
            )?;
            prepared.metadata = Some(metadata_from_lock_package(expected));
            selected_packages.push(current.clone());
            prepared_packages.push(prepared);
        }

        let registry = load_registry(&request.installation)?;
        let backup_path = Some(
            create_backup(BackupRequest {
                installation: request.installation.clone(),
                output_path: request.backup_output_path.clone(),
                groups: vec![BackupGroup::Addons],
                label: Some("addon-lock-apply-update".to_string()),
            })?
            .archive_path,
        );
        match update_prepared_packages(
            &request.installation,
            registry,
            selected_packages,
            prepared_packages,
        ) {
            Ok(_) => {}
            Err(error) => {
                return rollback_or_report_addon_error(
                    error,
                    backup_path.as_deref(),
                    &request.installation,
                );
            }
        }
    }

    for action in plan
        .actions
        .iter()
        .filter(|action| action.action.kind == AddonLockSyncActionKind::Install)
    {
        let expected = action.expected.as_ref().ok_or_else(|| {
            AppError::Validation("lock install action is missing expected package".to_string())
        })?;
        let prepared = prepare_expected_lock_package(
            expected,
            source_overrides
                .get(&action.action.comparison_key)
                .map(PathBuf::as_path),
            request.installation.flavor,
        )?;
        install_prepared_addon(InstallPreparedAddonRequest {
            installation: request.installation.clone(),
            prepared,
            dry_run: false,
            backup_output_path: request.backup_output_path.clone(),
            replace_existing: request.replace_existing,
            metadata: Some(metadata_from_lock_package(expected)),
        })?;
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

pub fn write_addon_lock(
    installation: &DetectedFlavorInstallation,
) -> AppResult<AddonLockWriteResult> {
    let registry = load_registry(installation)?;
    let path = lock_path(installation);
    if registry.packages.is_empty() {
        cleanup_addon_lock(&path)?;
        return Ok(AddonLockWriteResult {
            lock_path: path,
            package_count: 0,
            removed: true,
        });
    }

    let lock = build_addon_lock(installation, &registry)?;
    write_addon_lock_file(&path, &lock)?;

    Ok(AddonLockWriteResult {
        lock_path: path,
        package_count: lock.packages.len(),
        removed: false,
    })
}

pub(crate) fn sync_addon_lock_from_registry(
    installation: &DetectedFlavorInstallation,
    registry: &AddonRegistry,
) -> AppResult<()> {
    let path = lock_path(installation);
    if registry.packages.is_empty() {
        cleanup_addon_lock(&path)?;
        return Ok(());
    }

    let lock = build_addon_lock(installation, registry)?;
    write_addon_lock_file(&path, &lock)
}

pub fn lock_path(installation: &DetectedFlavorInstallation) -> PathBuf {
    installation.addon_dir.join(".hearthsync").join("lock.toml")
}

fn build_addon_lock(
    installation: &DetectedFlavorInstallation,
    registry: &AddonRegistry,
) -> AppResult<AddonLock> {
    let mut packages = registry
        .packages
        .iter()
        .map(|package| build_lock_package(installation, package))
        .collect::<AppResult<Vec<_>>>()?;
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));

    Ok(AddonLock {
        schema_version: 1,
        generated_at: now_rfc3339()?,
        packages,
    })
}

fn build_lock_package(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<AddonLockPackage> {
    let metadata = package.metadata.as_ref();
    let mut addons = package.addons.clone();
    addons.sort_by(|left, right| left.directory_name.cmp(&right.directory_name));
    let addon_directories = addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>();

    Ok(AddonLockPackage {
        package_id: package.package_id.clone(),
        index_name: metadata.and_then(|value| value.index_name.clone()),
        index_package_id: metadata.and_then(|value| value.index_package_id.clone()),
        name: lock_package_name(package, metadata),
        version: lock_package_version(package, metadata),
        source: package.source.clone(),
        source_url: metadata.and_then(|value| value.source_url.clone()),
        website_url: metadata.and_then(|value| value.website_url.clone()),
        source_sha256: metadata.and_then(|value| value.source_sha256.clone()),
        content_sha256: package_content_sha256(installation, package)?,
        installed_at: package.installed_at.clone(),
        updated_at: package.updated_at.clone(),
        addon_directories,
        addons,
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

fn metadata_from_lock_package(package: &AddonLockPackage) -> AddonPackageMetadata {
    AddonPackageMetadata {
        index_name: package.index_name.clone(),
        index_package_id: package.index_package_id.clone(),
        package_name: package.name.clone(),
        version: package.version.clone(),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.source_sha256.clone(),
        supported_flavors: Vec::new(),
    }
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

fn read_addon_lock(path: &Path) -> AppResult<AddonLock> {
    let content = fs::read_to_string(path)?;
    let lock = toml::from_str::<AddonLock>(&content)?;
    validate_addon_lock(&lock)?;
    Ok(lock)
}

fn lock_package_name(
    package: &TrackedAddonPackage,
    metadata: Option<&AddonPackageMetadata>,
) -> Option<String> {
    metadata
        .and_then(|value| value.package_name.clone())
        .or_else(|| {
            package
                .addons
                .iter()
                .filter_map(|addon| addon.title.clone())
                .find(|value| !value.trim().is_empty())
        })
        .or_else(|| Some(package.package_id.clone()))
}

fn lock_package_version(
    package: &TrackedAddonPackage,
    metadata: Option<&AddonPackageMetadata>,
) -> Option<String> {
    metadata
        .and_then(|value| value.version.clone())
        .or_else(|| infer_addon_version(&package.addons))
}

fn infer_addon_version(addons: &[TrackedAddon]) -> Option<String> {
    let versions = addons
        .iter()
        .filter_map(|addon| addon.version.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if versions.len() == 1 {
        versions.iter().next().map(|value| (*value).to_string())
    } else {
        None
    }
}

fn package_content_sha256(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<String> {
    let (content_sha256, missing_addon_directories) =
        package_content_sha256_with_missing(installation, package)?;
    if !missing_addon_directories.is_empty() {
        return Err(AppError::NotFound(format!(
            "tracked addon directories missing for package `{}`: {}",
            package.package_id,
            missing_addon_directories.join(", ")
        )));
    }
    Ok(content_sha256.unwrap_or_default())
}

fn package_content_sha256_with_missing(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<(Option<String>, Vec<String>)> {
    let mut files = Vec::new();
    let mut missing_addon_directories = Vec::new();
    for addon in &package.addons {
        let addon_path = installation.addon_dir.join(&addon.directory_name);
        if !addon_path.is_dir() {
            missing_addon_directories.push(addon.directory_name.clone());
            continue;
        }

        for entry in WalkDir::new(&addon_path) {
            let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative_path = normalize_relative_path(entry.path(), &installation.addon_dir)?;
            files.push((relative_path, entry.path().to_path_buf()));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    if !missing_addon_directories.is_empty() {
        return Ok((None, missing_addon_directories));
    }

    let mut hasher = Sha256::new();
    for (relative_path, path) in files {
        hash_file_entry(&mut hasher, &relative_path, &path)?;
    }

    Ok((
        Some(format!("{:x}", hasher.finalize())),
        missing_addon_directories,
    ))
}

fn hash_file_entry(hasher: &mut Sha256, relative_path: &str, path: &Path) -> AppResult<()> {
    let length = fs::metadata(path)?.len();
    hasher.update(relative_path.as_bytes());
    hasher.update([0]);
    hasher.update(length.to_le_bytes());
    hasher.update([0]);

    let mut file = File::open(path)?;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher.update([0]);

    Ok(())
}

fn normalize_relative_path(path: &Path, base: &Path) -> AppResult<String> {
    let relative = path.strip_prefix(base).map_err(|_| {
        AppError::Validation(format!(
            "path `{}` is outside addon root `{}`",
            path.display(),
            base.display()
        ))
    })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn write_addon_lock_file(path: &Path, lock: &AddonLock) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, toml::to_string_pretty(lock)?)?;
    Ok(())
}

fn cleanup_addon_lock(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if !parent.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(parent)?;
    if entries.next().is_none() {
        fs::remove_dir(parent)?;
    }

    Ok(())
}

fn validate_addon_lock(lock: &AddonLock) -> AppResult<()> {
    if lock.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon lock schema version: {}",
            lock.schema_version
        )));
    }

    let mut comparison_keys = BTreeSet::new();
    for package in &lock.packages {
        let comparison_key = comparison_key(
            &package.package_id,
            package.index_name.as_deref(),
            package.index_package_id.as_deref(),
            &package.addon_directories,
        );
        if !comparison_keys.insert(comparison_key.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate addon lock package comparison key: {comparison_key}"
            )));
        }
    }
    Ok(())
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn compare_lock_snapshots(
    left_label: &str,
    left_packages: &[AddonLockPackageSnapshot],
    right_label: &str,
    right_packages: &[AddonLockPackageSnapshot],
) -> AppResult<AddonLockDiffResult> {
    let left_map = snapshot_map(left_packages)?;
    let right_map = snapshot_map(right_packages)?;

    let mut all_keys = left_map.keys().cloned().collect::<Vec<_>>();
    for key in right_map.keys() {
        if !left_map.contains_key(key) {
            all_keys.push(key.clone());
        }
    }
    all_keys.sort();

    let mut unchanged_packages = 0usize;
    let mut added_packages = Vec::new();
    let mut removed_packages = Vec::new();
    let mut changed_packages = Vec::new();

    for key in all_keys {
        match (left_map.get(&key), right_map.get(&key)) {
            (Some(left), Some(right)) => {
                let changes = diff_snapshot_fields(left, right);
                if changes.is_empty() {
                    unchanged_packages += 1;
                } else {
                    changed_packages.push(AddonLockPackageDiff {
                        comparison_key: key,
                        left: (*left).clone(),
                        right: (*right).clone(),
                        changes,
                    });
                }
            }
            (Some(left), None) => removed_packages.push((*left).clone()),
            (None, Some(right)) => added_packages.push((*right).clone()),
            (None, None) => {}
        }
    }

    Ok(AddonLockDiffResult {
        left_label: left_label.to_string(),
        right_label: right_label.to_string(),
        left_package_count: left_packages.len(),
        right_package_count: right_packages.len(),
        identical: added_packages.is_empty()
            && removed_packages.is_empty()
            && changed_packages.is_empty(),
        unchanged_packages,
        added_packages,
        removed_packages,
        changed_packages,
    })
}

fn snapshot_map<'a>(
    packages: &'a [AddonLockPackageSnapshot],
) -> AppResult<std::collections::BTreeMap<String, &'a AddonLockPackageSnapshot>> {
    let mut map = std::collections::BTreeMap::new();
    for package in packages {
        if map
            .insert(package.comparison_key.clone(), package)
            .is_some()
        {
            return Err(AppError::Validation(format!(
                "duplicate addon lock snapshot comparison key: {}",
                package.comparison_key
            )));
        }
    }
    Ok(map)
}

fn lock_snapshots(lock: &AddonLock) -> AppResult<Vec<AddonLockPackageSnapshot>> {
    let mut packages = lock
        .packages
        .iter()
        .map(snapshot_from_lock_package)
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left.comparison_key.cmp(&right.comparison_key));
    if has_duplicate_snapshot_keys(&packages) {
        return Err(AppError::Validation(
            "duplicate addon lock package comparison keys".to_string(),
        ));
    }
    Ok(packages)
}

fn snapshot_from_lock_package(package: &AddonLockPackage) -> AddonLockPackageSnapshot {
    AddonLockPackageSnapshot {
        comparison_key: comparison_key(
            &package.package_id,
            package.index_name.as_deref(),
            package.index_package_id.as_deref(),
            &package.addon_directories,
        ),
        package_id: package.package_id.clone(),
        index_name: package.index_name.clone(),
        index_package_id: package.index_package_id.clone(),
        name: package.name.clone(),
        version: package.version.clone(),
        source: package.source.clone(),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.source_sha256.clone(),
        content_sha256: Some(package.content_sha256.clone()),
        addon_directories: package.addon_directories.clone(),
    }
}

fn snapshot_from_tracked_package(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> (AddonLockPackageSnapshot, Vec<String>) {
    let metadata = package.metadata.as_ref();
    let (content_sha256, missing_addon_directories) =
        package_content_sha256_with_missing(installation, package).unwrap_or_else(|_| {
            (
                None,
                package
                    .addons
                    .iter()
                    .map(|addon| addon.directory_name.clone())
                    .collect(),
            )
        });

    let mut addon_directories = package
        .addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>();
    addon_directories.sort();

    (
        AddonLockPackageSnapshot {
            comparison_key: comparison_key(
                &package.package_id,
                metadata.and_then(|value| value.index_name.as_deref()),
                metadata.and_then(|value| value.index_package_id.as_deref()),
                &addon_directories,
            ),
            package_id: package.package_id.clone(),
            index_name: metadata.and_then(|value| value.index_name.clone()),
            index_package_id: metadata.and_then(|value| value.index_package_id.clone()),
            name: lock_package_name(package, metadata),
            version: lock_package_version(package, metadata),
            source: package.source.clone(),
            source_url: metadata.and_then(|value| value.source_url.clone()),
            website_url: metadata.and_then(|value| value.website_url.clone()),
            source_sha256: metadata.and_then(|value| value.source_sha256.clone()),
            content_sha256,
            addon_directories,
        },
        missing_addon_directories,
    )
}

fn diff_snapshot_fields(
    left: &AddonLockPackageSnapshot,
    right: &AddonLockPackageSnapshot,
) -> Vec<AddonLockFieldChange> {
    let mut changes = Vec::new();
    push_change(
        &mut changes,
        "package_id",
        Some(left.package_id.clone()),
        Some(right.package_id.clone()),
    );
    push_change(&mut changes, "name", left.name.clone(), right.name.clone());
    push_change(
        &mut changes,
        "version",
        left.version.clone(),
        right.version.clone(),
    );
    push_change(
        &mut changes,
        "source",
        Some(left.source.display_name()),
        Some(right.source.display_name()),
    );
    push_change(
        &mut changes,
        "source_url",
        left.source_url.clone(),
        right.source_url.clone(),
    );
    push_change(
        &mut changes,
        "website_url",
        left.website_url.clone(),
        right.website_url.clone(),
    );
    push_change(
        &mut changes,
        "source_sha256",
        left.source_sha256.clone(),
        right.source_sha256.clone(),
    );
    push_change(
        &mut changes,
        "content_sha256",
        left.content_sha256.clone(),
        right.content_sha256.clone(),
    );
    push_change(
        &mut changes,
        "addon_directories",
        Some(left.addon_directories.join(", ")),
        Some(right.addon_directories.join(", ")),
    );
    changes
}

fn push_change(
    changes: &mut Vec<AddonLockFieldChange>,
    field: &str,
    left: Option<String>,
    right: Option<String>,
) {
    if left != right {
        changes.push(AddonLockFieldChange {
            field: field.to_string(),
            left,
            right,
        });
    }
}

fn has_duplicate_snapshot_keys(packages: &[AddonLockPackageSnapshot]) -> bool {
    let mut keys = BTreeSet::new();
    packages
        .iter()
        .any(|package| !keys.insert(package.comparison_key.clone()))
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    use tempfile::tempdir;
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        AddonLockApplyRequest, apply_addon_lock_sync, diff_addon_locks, inspect_addon_lock,
        lock_path, plan_addon_lock_sync, verify_addon_lock, write_addon_lock,
    };
    use crate::core::addon::{
        AddonPackageMetadata, InstallAddonRequest, RemoveAddonRequest, install_addon, remove_addons,
    };
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

    #[test]
    fn install_addon_writes_lock_with_metadata_and_content_hash() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let archive_path = temp.path().join("details.zip");
        create_addon_archive(
            &archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Title: Details!\n## Version: 1.0.0\n",
            )],
        );

        install_addon(InstallAddonRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: Some(AddonPackageMetadata {
                index_name: Some("Fixture Index".to_string()),
                index_package_id: Some("details".to_string()),
                package_name: Some("Details".to_string()),
                version: Some("1.0.0".to_string()),
                source_url: Some("https://example.com/details.zip".to_string()),
                website_url: Some("https://example.com/details".to_string()),
                source_sha256: Some("source-hash".to_string()),
                supported_flavors: vec!["retail".to_string()],
            }),
        })
        .expect("install addon");

        let inspection = inspect_addon_lock(&installation).expect("inspect lock");
        assert_eq!(inspection.package_count, 1);
        assert_eq!(
            inspection.lock.packages[0].index_package_id.as_deref(),
            Some("details")
        );
        assert_eq!(inspection.lock.packages[0].name.as_deref(), Some("Details"));
        assert_eq!(
            inspection.lock.packages[0].version.as_deref(),
            Some("1.0.0")
        );
        assert_eq!(inspection.lock.packages[0].content_sha256.len(), 64);
        assert_eq!(
            inspection.lock.packages[0].addon_directories,
            vec!["Details"]
        );
    }

    #[test]
    fn write_addon_lock_removes_stale_lock_when_registry_is_empty() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let path = lock_path(&installation);
        fs::create_dir_all(path.parent().expect("lock parent")).expect("lock parent");
        fs::write(&path, "stale").expect("stale lock");

        let result = write_addon_lock(&installation).expect("write lock");

        assert!(result.removed);
        assert!(!path.exists());
    }

    #[test]
    fn remove_addon_cleans_lock_file_when_last_package_is_removed() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let archive_path = temp.path().join("details.zip");
        create_addon_archive(
            &archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );

        install_addon(InstallAddonRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");
        assert!(lock_path(&installation).exists());

        remove_addons(RemoveAddonRequest {
            installation: installation.clone(),
            name: "Details".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        })
        .expect("remove addon");

        assert!(!lock_path(&installation).exists());
    }

    #[test]
    fn diff_addon_locks_reports_changed_added_and_removed_packages() {
        let temp = tempdir().expect("temp dir");
        let left_path = temp.path().join("left.toml");
        let right_path = temp.path().join("right.toml");

        fs::write(
            &left_path,
            r#"
schema_version = 1
generated_at = "2026-04-15T00:00:00Z"

[[packages]]
package_id = "details"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "1.0.0"
source = { kind = "local_archive", path = "C:\\details.zip" }
content_sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "omen"
name = "Omen"
source = { kind = "local_archive", path = "C:\\omen.zip" }
content_sha256 = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
installed_at = "2026-04-15T00:00:00Z"
updated_at = "2026-04-15T00:00:00Z"
addon_directories = ["Omen"]
addons = []
"#,
        )
        .expect("left lock");

        fs::write(
            &right_path,
            r#"
schema_version = 1
generated_at = "2026-04-16T00:00:00Z"

[[packages]]
package_id = "details-v2"
index_name = "Raid"
index_package_id = "details"
name = "Details"
version = "2.0.0"
source = { kind = "local_archive", path = "C:\\details-v2.zip" }
content_sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["Details"]
addons = []

[[packages]]
package_id = "bigwigs"
name = "BigWigs"
source = { kind = "local_archive", path = "C:\\bigwigs.zip" }
content_sha256 = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
installed_at = "2026-04-16T00:00:00Z"
updated_at = "2026-04-16T00:00:00Z"
addon_directories = ["BigWigs"]
addons = []
"#,
        )
        .expect("right lock");

        let diff = diff_addon_locks(&left_path, &right_path).expect("diff locks");

        assert!(!diff.identical);
        assert_eq!(diff.changed_packages.len(), 1);
        assert_eq!(diff.added_packages.len(), 1);
        assert_eq!(diff.removed_packages.len(), 1);
        assert!(
            diff.changed_packages[0]
                .changes
                .iter()
                .any(|change| change.field == "version")
        );
    }

    #[test]
    fn verify_addon_lock_reports_drift_and_untracked_addons() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let archive_path = temp.path().join("details.zip");
        create_addon_archive(
            &archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );

        install_addon(InstallAddonRequest {
            installation: installation.clone(),
            source: archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install addon");

        fs::write(
            installation.addon_dir.join("Details").join("Details.toc"),
            "## Interface: 110000\n## Version: 2.0.0\n",
        )
        .expect("mutate toc");
        fs::create_dir_all(installation.addon_dir.join("BigWigs")).expect("untracked addon dir");
        fs::write(
            installation.addon_dir.join("BigWigs").join("BigWigs.toc"),
            "## Interface: 110000\n## Version: 1.0.0\n",
        )
        .expect("untracked addon toc");

        let verification = verify_addon_lock(&installation, None).expect("verify lock");

        assert!(!verification.matches);
        assert_eq!(verification.diff.changed_packages.len(), 1);
        assert_eq!(verification.untracked_addons, vec!["BigWigs"]);
        assert!(
            verification.diff.changed_packages[0]
                .changes
                .iter()
                .any(|change| change.field == "content_sha256")
        );
    }

    #[test]
    fn apply_addon_lock_sync_updates_installs_and_removes_packages() {
        let temp = tempdir().expect("temp dir");
        let source_root = temp.path().join("sources");
        fs::create_dir_all(&source_root).expect("source root");

        let details_v1 = source_root.join("details-v1.zip");
        let details_v2 = source_root.join("details-v2.zip");
        let omen = source_root.join("omen.zip");
        let bigwigs = source_root.join("bigwigs.zip");
        create_addon_archive(
            &details_v1,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );
        create_addon_archive(
            &details_v2,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Version: 2.0.0\n",
            )],
        );
        create_addon_archive(
            &omen,
            &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
        );
        create_addon_archive(
            &bigwigs,
            &[(
                "BigWigs/BigWigs.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );

        let desired_installation = create_fixture_installation(&temp.path().join("desired"));
        install_addon(InstallAddonRequest {
            installation: desired_installation.clone(),
            source: details_v2.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("desired-backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install desired details");
        install_addon(InstallAddonRequest {
            installation: desired_installation.clone(),
            source: bigwigs.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("desired-backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install desired bigwigs");
        let desired_lock = write_addon_lock(&desired_installation)
            .expect("write desired lock")
            .lock_path;

        let current_installation = create_fixture_installation(&temp.path().join("current"));
        install_addon(InstallAddonRequest {
            installation: current_installation.clone(),
            source: details_v1.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("current-backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install current details");
        install_addon(InstallAddonRequest {
            installation: current_installation.clone(),
            source: omen.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("current-backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install current omen");

        let plan = plan_addon_lock_sync(&current_installation, Some(&desired_lock)).expect("plan");
        assert_eq!(plan.install_count, 1);
        assert_eq!(plan.update_count, 1);
        assert_eq!(plan.remove_count, 1);
        assert_eq!(plan.blocked_count, 0);

        let result = apply_addon_lock_sync(AddonLockApplyRequest {
            installation: current_installation.clone(),
            lock_path: Some(desired_lock.clone()),
            backup_output_path: Some(temp.path().join("apply-backups")),
            replace_existing: false,
            source_overrides: Vec::new(),
        })
        .expect("apply lock sync");

        assert!(result.verification.matches);
        assert_eq!(result.install_count, 1);
        assert_eq!(result.update_count, 1);
        assert_eq!(result.remove_count, 1);
        assert!(
            fs::read_to_string(
                current_installation
                    .addon_dir
                    .join("Details")
                    .join("Details.toc")
            )
            .expect("details toc")
            .contains("2.0.0")
        );
        assert!(current_installation.addon_dir.join("BigWigs").exists());
        assert!(!current_installation.addon_dir.join("Omen").exists());
    }

    fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
        let product_root = root.join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        fs::create_dir_all(&addon_dir).expect("addon dir");
        fs::create_dir_all(&wtf_dir).expect("wtf dir");
        fs::create_dir_all(&fonts_dir).expect("fonts dir");

        DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root,
            flavor_root,
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        }
    }

    fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
        let file = fs::File::create(path).expect("archive file");
        let mut zip = ZipWriter::new(file);
        for (name, content) in entries {
            zip.start_file(
                name.replace('\\', "/"),
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .expect("start file");
            zip.write_all(content.as_bytes()).expect("write file");
        }
        zip.finish().expect("finish zip");
    }
}
