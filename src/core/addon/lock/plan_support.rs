use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::core::addon::{
    AddonInventory, AddonSourceRef, TrackedAddonPackage, addon_directory_path_key,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

use super::{
    AddonLock, AddonLockFieldChange, AddonLockPackage, AddonLockPackageDirectoryIssue,
    AddonLockSyncActionKind, comparison_key,
};

pub(super) fn expected_package_map(
    lock: &AddonLock,
) -> AppResult<BTreeMap<String, AddonLockPackage>> {
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

pub(super) fn current_package_map(
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

pub(super) fn missing_directory_map(
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

pub(super) fn tracked_directory_owner_map(
    inventory: &AddonInventory,
    platform: HostPlatform,
) -> BTreeMap<String, String> {
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
            map.insert(
                addon_directory_path_key(&addon.directory_name, platform),
                key.clone(),
            );
        }
    }
    map
}

pub(super) fn freed_directory_set(
    current_packages: &BTreeMap<String, TrackedAddonPackage>,
    remove_keys: &BTreeSet<String>,
    update_keys: &BTreeSet<String>,
    platform: HostPlatform,
) -> BTreeSet<String> {
    let mut freed = BTreeSet::new();
    for key in remove_keys.iter().chain(update_keys.iter()) {
        if let Some(package) = current_packages.get(key) {
            for addon in &package.addons {
                freed.insert(addon_directory_path_key(&addon.directory_name, platform));
            }
        }
    }
    freed
}

pub(super) fn preflight_expected_source(
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

pub(super) fn directory_conflicts(
    comparison_key: &str,
    addon_directories: &[String],
    occupied_by_tracked: &BTreeMap<String, String>,
    freed_directories: &BTreeSet<String>,
    untracked_addons: &BTreeSet<String>,
    platform: HostPlatform,
) -> (Vec<String>, bool) {
    let mut blocked_reasons = Vec::new();
    let mut requires_replace_existing = false;

    for addon_directory in addon_directories {
        let normalized = addon_directory_path_key(addon_directory, platform);
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

pub(super) fn has_material_changes(changes: &[AddonLockFieldChange]) -> bool {
    changes.iter().any(|change| {
        matches!(
            change.field.as_str(),
            "source" | "content_sha256" | "addon_directories"
        )
    })
}

pub(super) fn lock_action_sort_key(kind: &AddonLockSyncActionKind) -> u8 {
    match kind {
        AddonLockSyncActionKind::Remove => 0,
        AddonLockSyncActionKind::Update => 1,
        AddonLockSyncActionKind::Install => 2,
        AddonLockSyncActionKind::MetadataOnly => 3,
    }
}

fn preflight_source_ref(source: &AddonSourceRef) -> Vec<String> {
    match source {
        AddonSourceRef::LocalArchive { path } if !path.is_absolute() => {
            vec![format!(
                "local archive path must be absolute before lock planning: {}",
                path.display()
            )]
        }
        AddonSourceRef::LocalArchive { path } if !path.is_file() => {
            vec![format!(
                "local archive is not available: {}",
                path.display()
            )]
        }
        _ => Vec::new(),
    }
}
