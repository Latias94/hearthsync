use std::collections::BTreeSet;

use crate::core::addon::{PreparedAddonPackage, TrackedAddonPackage};
use crate::core::error::{AppError, AppResult};

use super::AddonIndexPackage;

pub(super) fn match_index_package_to_tracked_package(
    package: &AddonIndexPackage,
    prepared: &PreparedAddonPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<TrackedAddonPackage> {
    let expected_addon_names = expected_addon_names(package, prepared);

    let exact_id_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && candidate.package_id.eq_ignore_ascii_case(&package.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    if exact_id_matches.len() == 1 {
        return Ok(exact_id_matches[0].clone());
    }
    if exact_id_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by id",
            package.id
        )));
    }

    let full_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && tracked_package_contains_all_addons(candidate, &expected_addon_names)
        })
        .cloned()
        .collect::<Vec<_>>();
    if full_matches.len() == 1 {
        return Ok(full_matches[0].clone());
    }
    if full_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by addon directories: {}",
            package.id,
            full_matches
                .iter()
                .map(|candidate| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let mut partial_matches = tracked_packages
        .iter()
        .filter(|candidate| !used_package_ids.contains(&candidate.package_id))
        .filter_map(|candidate| {
            let overlap = tracked_package_addon_overlap(candidate, &expected_addon_names);
            (overlap > 0).then_some((overlap, candidate.clone()))
        })
        .collect::<Vec<_>>();
    partial_matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.package_id.cmp(&right.1.package_id))
    });

    match partial_matches.as_slice() {
        [] => Err(AppError::Validation(format!(
            "addon index package `{}` is not installed or not tracked locally",
            package.id
        ))),
        [(_, candidate)] => Ok(candidate.clone()),
        [(best_overlap, best), (next_overlap, next), ..] if best_overlap > next_overlap => {
            let _ = next;
            Ok(best.clone())
        }
        _ => Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages with the same confidence: {}",
            package.id,
            partial_matches
                .iter()
                .map(|(_, candidate)| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn expected_addon_names(
    package: &AddonIndexPackage,
    prepared: &PreparedAddonPackage,
) -> BTreeSet<String> {
    let addon_names = if package.addon_directories.is_empty() {
        prepared
            .addons
            .iter()
            .map(|addon| addon.addon.directory_name.clone())
            .collect::<Vec<_>>()
    } else {
        package.addon_directories.clone()
    };

    addon_names
        .into_iter()
        .map(|name| name.trim().to_ascii_lowercase())
        .filter(|name| !name.is_empty())
        .collect()
}

fn tracked_package_contains_all_addons(
    candidate: &TrackedAddonPackage,
    expected_addon_names: &BTreeSet<String>,
) -> bool {
    !expected_addon_names.is_empty()
        && candidate
            .addons
            .iter()
            .map(|addon| addon.directory_name.trim().to_ascii_lowercase())
            .collect::<BTreeSet<_>>()
            .is_superset(expected_addon_names)
}

fn tracked_package_addon_overlap(
    candidate: &TrackedAddonPackage,
    expected_addon_names: &BTreeSet<String>,
) -> usize {
    candidate
        .addons
        .iter()
        .map(|addon| addon.directory_name.trim().to_ascii_lowercase())
        .filter(|name| expected_addon_names.contains(name))
        .count()
}
