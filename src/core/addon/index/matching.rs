use std::collections::BTreeSet;

use crate::core::addon::{PreparedAddonPackage, TrackedAddonPackage};
use crate::core::error::{AppError, AppResult};

use super::{AddonIndexPackage, AddonIndexTrackedMatchStrategy};

mod strategies;

pub(super) use self::strategies::package_id_usage_key;
use self::strategies::{
    expected_addon_names, explicit_addon_names, match_by_curated_package_id_hints,
    match_by_display_name, match_by_exact_package_id, match_by_full_addon_names,
    match_by_index_package_metadata, match_by_source_family_identity, match_by_source_identity,
    package_id_is_used, tracked_package_addon_overlap,
};

#[derive(Debug, Clone)]
pub(super) struct ExplainedTrackedPackageMatch {
    pub(super) package: TrackedAddonPackage,
    pub(super) strategy: AddonIndexTrackedMatchStrategy,
}

pub(super) fn match_index_package_to_tracked_package(
    package: &AddonIndexPackage,
    prepared: &PreparedAddonPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<TrackedAddonPackage> {
    if let Some(matched) =
        match_by_index_package_metadata(package, tracked_packages, used_package_ids)?
    {
        return Ok(matched.package);
    }

    if let Some(matched) = match_by_exact_package_id(package, tracked_packages, used_package_ids)? {
        return Ok(matched.package);
    }

    if let Some(matched) =
        match_by_curated_package_id_hints(package, tracked_packages, used_package_ids)?
    {
        return Ok(matched.package);
    }

    if let Some(matched) =
        match_by_source_identity(&package.source, tracked_packages, used_package_ids)?
    {
        return Ok(matched.package);
    }

    if let Some(matched) =
        match_by_source_family_identity(&package.source, tracked_packages, used_package_ids)?
    {
        return Ok(matched.package);
    }

    if let Some(matched) = match_by_display_name(package, tracked_packages, used_package_ids)? {
        return Ok(matched.package);
    }

    let expected_addon_names = expected_addon_names(package, prepared);

    if let Some(matched) = match_by_full_addon_names(
        package,
        tracked_packages,
        used_package_ids,
        &expected_addon_names,
    )? {
        return Ok(matched.package);
    }

    let mut partial_matches = tracked_packages
        .iter()
        .filter(|candidate| !package_id_is_used(used_package_ids, &candidate.package_id))
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

pub(super) fn preflight_match_index_package_to_tracked_package(
    package: &AddonIndexPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<TrackedAddonPackage>> {
    Ok(explain_preflight_match_index_package_to_tracked_package(
        package,
        tracked_packages,
        used_package_ids,
    )?
    .map(|matched| matched.package))
}

pub(super) fn explain_preflight_match_index_package_to_tracked_package(
    package: &AddonIndexPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    if let Some(matched) =
        match_by_index_package_metadata(package, tracked_packages, used_package_ids)?
    {
        return Ok(Some(matched));
    }

    if let Some(matched) = match_by_exact_package_id(package, tracked_packages, used_package_ids)? {
        return Ok(Some(matched));
    }

    if let Some(matched) =
        match_by_curated_package_id_hints(package, tracked_packages, used_package_ids)?
    {
        return Ok(Some(matched));
    }

    if let Some(matched) =
        match_by_source_identity(&package.source, tracked_packages, used_package_ids)?
    {
        return Ok(Some(matched));
    }

    if let Some(matched) =
        match_by_source_family_identity(&package.source, tracked_packages, used_package_ids)?
    {
        return Ok(Some(matched));
    }

    if let Some(matched) = match_by_display_name(package, tracked_packages, used_package_ids)? {
        return Ok(Some(matched));
    }

    let expected_addon_names = explicit_addon_names(package);
    if expected_addon_names.is_empty() {
        return Ok(None);
    }

    match_by_full_addon_names(
        package,
        tracked_packages,
        used_package_ids,
        &expected_addon_names,
    )
}
