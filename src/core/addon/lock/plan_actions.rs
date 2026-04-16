use super::plan_model::PlannedLockAction;
use super::plan_support::{
    directory_conflicts, has_material_changes, lock_action_sort_key, preflight_expected_source,
};
use super::*;

pub(super) struct ActionBuildInputs<'a> {
    pub(super) diff: &'a AddonLockDiffResult,
    pub(super) expected_packages: &'a BTreeMap<String, AddonLockPackage>,
    pub(super) current_packages: &'a BTreeMap<String, TrackedAddonPackage>,
    pub(super) missing_map: &'a BTreeMap<String, Vec<String>>,
    pub(super) source_overrides: &'a BTreeMap<String, PathBuf>,
    pub(super) occupied_by_tracked: &'a BTreeMap<String, String>,
    pub(super) freed_directories: &'a BTreeSet<String>,
    pub(super) untracked_addons: &'a BTreeSet<String>,
}

pub(super) fn build_plan_actions(
    inputs: ActionBuildInputs<'_>,
) -> AppResult<Vec<PlannedLockAction>> {
    let mut actions = Vec::new();

    for package in &inputs.diff.removed_packages {
        actions.push(build_install_action(package, &inputs)?);
    }

    for package in &inputs.diff.added_packages {
        actions.push(build_remove_action(package, &inputs)?);
    }

    for package in &inputs.diff.changed_packages {
        actions.push(build_changed_action(package, &inputs)?);
    }

    actions.sort_by(|left, right| {
        lock_action_sort_key(&left.action.kind)
            .cmp(&lock_action_sort_key(&right.action.kind))
            .then_with(|| left.action.comparison_key.cmp(&right.action.comparison_key))
    });

    Ok(actions)
}

fn build_install_action(
    package: &AddonLockPackageSnapshot,
    inputs: &ActionBuildInputs<'_>,
) -> AppResult<PlannedLockAction> {
    let expected = inputs
        .expected_packages
        .get(&package.comparison_key)
        .cloned()
        .ok_or_else(|| AppError::Validation("expected lock package missing".to_string()))?;
    let blocked_reasons = preflight_expected_source(
        &package.comparison_key,
        &expected.source,
        inputs.source_overrides,
    );
    let (conflict_reasons, requires_replace_existing) = directory_conflicts(
        &package.comparison_key,
        &package.addon_directories,
        inputs.occupied_by_tracked,
        inputs.freed_directories,
        inputs.untracked_addons,
    );

    Ok(PlannedLockAction {
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
                .chain(conflict_reasons)
                .collect(),
            requires_replace_existing,
        },
        expected: Some(expected),
        current: None,
    })
}

fn build_remove_action(
    package: &AddonLockPackageSnapshot,
    inputs: &ActionBuildInputs<'_>,
) -> AppResult<PlannedLockAction> {
    let current = inputs
        .current_packages
        .get(&package.comparison_key)
        .cloned()
        .ok_or_else(|| AppError::Validation("current tracked package missing".to_string()))?;

    Ok(PlannedLockAction {
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
    })
}

fn build_changed_action(
    package: &AddonLockPackageDiff,
    inputs: &ActionBuildInputs<'_>,
) -> AppResult<PlannedLockAction> {
    let expected = inputs
        .expected_packages
        .get(&package.comparison_key)
        .cloned()
        .ok_or_else(|| AppError::Validation("expected lock package missing".to_string()))?;
    let current = inputs
        .current_packages
        .get(&package.comparison_key)
        .cloned()
        .ok_or_else(|| AppError::Validation("current tracked package missing".to_string()))?;
    let has_missing = inputs
        .missing_map
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
    if let Some(missing) = inputs.missing_map.get(&package.comparison_key) {
        if !missing.is_empty() {
            reasons.push(format!(
                "tracked addon directories are missing: {}",
                missing.join(", ")
            ));
        }
    }
    let blocked_reasons = if kind == AddonLockSyncActionKind::Update {
        preflight_expected_source(
            &package.comparison_key,
            &expected.source,
            inputs.source_overrides,
        )
    } else {
        Vec::new()
    };
    let (conflict_reasons, requires_replace_existing) = if kind == AddonLockSyncActionKind::Update {
        directory_conflicts(
            &package.comparison_key,
            &expected.addon_directories,
            inputs.occupied_by_tracked,
            inputs.freed_directories,
            inputs.untracked_addons,
        )
    } else {
        (Vec::new(), false)
    };

    Ok(PlannedLockAction {
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
                .chain(conflict_reasons)
                .collect(),
            requires_replace_existing,
        },
        expected: Some(expected),
        current: Some(current),
    })
}
