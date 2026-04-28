use super::storage::{
    lock_package_name, lock_package_version, package_content_sha256_with_missing, read_addon_lock,
};
use std::collections::BTreeSet;
use std::path::Path;

use crate::core::addon::AddonStatePaths;
use crate::core::addon::TrackedAddonPackage;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

use super::{
    AddonLock, AddonLockDiffResult, AddonLockFieldChange, AddonLockPackage, AddonLockPackageDiff,
    AddonLockPackageDirectoryIssue, AddonLockPackageSnapshot, AddonLockVerifyResult,
    comparison_key, left_label, lock_path,
};

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
    state_paths: &AddonStatePaths,
    expected_lock_path: Option<&Path>,
) -> AppResult<AddonLockVerifyResult> {
    let lock_path = expected_lock_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| lock_path(state_paths));
    let expected = read_addon_lock(&lock_path)?;
    let inventory = crate::core::addon::list_addons(installation, state_paths)?;

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
pub(super) fn compare_lock_snapshots(
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

pub(super) fn lock_snapshots(lock: &AddonLock) -> AppResult<Vec<AddonLockPackageSnapshot>> {
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

pub(super) fn snapshot_from_tracked_package(
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
