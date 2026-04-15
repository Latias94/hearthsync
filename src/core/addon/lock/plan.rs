use super::source_resolution::resolved_source_override_map;
use super::storage::read_addon_lock;
use super::verify::{compare_lock_snapshots, lock_snapshots, snapshot_from_tracked_package};
use super::*;

#[derive(Debug, Clone)]
pub(super) struct PlannedLockAction {
    pub(super) action: AddonLockSyncAction,
    pub(super) expected: Option<AddonLockPackage>,
    pub(super) current: Option<TrackedAddonPackage>,
}

#[derive(Debug, Clone)]
pub(super) struct AddonLockPlanContext {
    pub(super) result: AddonLockPlanResult,
    pub(super) actions: Vec<PlannedLockAction>,
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

fn lock_action_sort_key(kind: &AddonLockSyncActionKind) -> u8 {
    match kind {
        AddonLockSyncActionKind::Remove => 0,
        AddonLockSyncActionKind::Update => 1,
        AddonLockSyncActionKind::Install => 2,
        AddonLockSyncActionKind::MetadataOnly => 3,
    }
}

pub(super) fn build_addon_lock_plan(
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
