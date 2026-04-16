use super::plan_actions::{ActionBuildInputs, build_plan_actions};
pub(super) use super::plan_model::AddonLockPlanContext;
use super::plan_model::PlannedLockAction;
use super::plan_support::{
    current_package_map, expected_package_map, freed_directory_set, has_material_changes,
    missing_directory_map, tracked_directory_owner_map,
};
use super::source_resolution::resolved_source_override_map;
use super::storage::read_addon_lock;
use super::verify::{compare_lock_snapshots, lock_snapshots, snapshot_from_tracked_package};
use super::*;

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
    let missing_map = missing_directory_map(&missing_addon_directories);
    let update_keys = collect_update_keys(&diff, &missing_map);
    let occupied_by_tracked = tracked_directory_owner_map(&inventory);
    let freed_directories = freed_directory_set(&current_packages, &remove_keys, &update_keys);
    let untracked_addons = inventory
        .untracked_addons
        .iter()
        .map(|name| name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    let actions = build_plan_actions(ActionBuildInputs {
        diff: &diff,
        expected_packages: &expected_packages,
        current_packages: &current_packages,
        missing_map: &missing_map,
        source_overrides: &source_overrides,
        occupied_by_tracked: &occupied_by_tracked,
        freed_directories: &freed_directories,
        untracked_addons: &untracked_addons,
    })?;

    Ok(build_plan_context(
        lock_path,
        installation.flavor_root.clone(),
        diff.unchanged_packages,
        inventory.untracked_addons,
        actions,
    ))
}

fn collect_update_keys(
    diff: &AddonLockDiffResult,
    missing_map: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    let mut update_keys = BTreeSet::new();
    for package in &diff.changed_packages {
        let has_missing = missing_map.contains_key(&package.comparison_key);
        if has_missing || has_material_changes(&package.changes) {
            update_keys.insert(package.comparison_key.clone());
        }
    }
    update_keys
}

fn build_plan_context(
    lock_path: PathBuf,
    installation_root: PathBuf,
    unchanged_count: usize,
    untracked_addons: Vec<String>,
    actions: Vec<PlannedLockAction>,
) -> AddonLockPlanContext {
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

    AddonLockPlanContext {
        result: AddonLockPlanResult {
            lock_path,
            installation_root,
            install_count,
            update_count,
            remove_count,
            metadata_only_count,
            unchanged_count,
            blocked_count,
            untracked_addons,
            actions: actions.iter().map(|action| action.action.clone()).collect(),
        },
        actions,
    }
}
