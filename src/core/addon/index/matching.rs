use std::collections::BTreeSet;

use crate::core::addon::{AddonSourceRef, PreparedAddonPackage, TrackedAddonPackage};
use crate::core::error::{AppError, AppResult};

use super::{AddonIndexPackage, AddonIndexTrackedMatchStrategy};

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

fn match_by_index_package_metadata(
    package: &AddonIndexPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    let metadata_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && candidate
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.index_package_id.as_deref())
                    .is_some_and(|index_package_id| {
                        index_package_id.eq_ignore_ascii_case(&package.id)
                    })
        })
        .cloned()
        .collect::<Vec<_>>();
    if metadata_matches.len() == 1 {
        return Ok(Some(ExplainedTrackedPackageMatch {
            package: metadata_matches[0].clone(),
            strategy: AddonIndexTrackedMatchStrategy::StoredIndexPackageId,
        }));
    }
    if metadata_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by stored index package id",
            package.id
        )));
    }

    Ok(None)
}

fn match_by_exact_package_id(
    package: &AddonIndexPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    let exact_id_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && candidate.package_id.eq_ignore_ascii_case(&package.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    if exact_id_matches.len() == 1 {
        return Ok(Some(ExplainedTrackedPackageMatch {
            package: exact_id_matches[0].clone(),
            strategy: AddonIndexTrackedMatchStrategy::ExactPackageId,
        }));
    }
    if exact_id_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by id",
            package.id
        )));
    }

    Ok(None)
}

fn match_by_curated_package_id_hints(
    package: &AddonIndexPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    let hinted_package_ids = normalized_package_ids(&package.match_package_ids);
    if hinted_package_ids.is_empty() {
        return Ok(None);
    }

    let hint_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && hinted_package_ids.contains(&candidate.package_id.trim().to_ascii_lowercase())
        })
        .cloned()
        .collect::<Vec<_>>();
    if hint_matches.len() == 1 {
        return Ok(Some(ExplainedTrackedPackageMatch {
            package: hint_matches[0].clone(),
            strategy: AddonIndexTrackedMatchStrategy::CuratedMatchPackageId,
        }));
    }
    if hint_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by curated match package ids: {}",
            package.id,
            hint_matches
                .iter()
                .map(|candidate| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(None)
}

fn match_by_source_identity(
    source: &AddonSourceRef,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    let source_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && tracked_package_has_same_source_identity(candidate, source)
        })
        .cloned()
        .collect::<Vec<_>>();
    if source_matches.len() == 1 {
        return Ok(Some(ExplainedTrackedPackageMatch {
            package: source_matches[0].clone(),
            strategy: AddonIndexTrackedMatchStrategy::SourceIdentity,
        }));
    }
    if source_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index source `{}` matched multiple tracked packages by source identity: {}",
            source.display_name(),
            source_matches
                .iter()
                .map(|candidate| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(None)
}

fn match_by_source_family_identity(
    source: &AddonSourceRef,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    let source_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && tracked_package_has_same_source_family_identity(candidate, source)
        })
        .cloned()
        .collect::<Vec<_>>();
    if source_matches.len() == 1 {
        return Ok(Some(ExplainedTrackedPackageMatch {
            package: source_matches[0].clone(),
            strategy: AddonIndexTrackedMatchStrategy::SourceFamilyIdentity,
        }));
    }
    if source_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index source `{}` matched multiple tracked packages by source family identity: {}",
            source.display_name(),
            source_matches
                .iter()
                .map(|candidate| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(None)
}

fn match_by_display_name(
    package: &AddonIndexPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    let package_name = package.name.trim();
    if package_name.is_empty() {
        return Ok(None);
    }

    let display_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && tracked_package_matches_display_name(candidate, package_name)
        })
        .cloned()
        .collect::<Vec<_>>();
    if display_matches.len() == 1 {
        return Ok(Some(ExplainedTrackedPackageMatch {
            package: display_matches[0].clone(),
            strategy: AddonIndexTrackedMatchStrategy::DisplayName,
        }));
    }
    if display_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by display name `{}`: {}",
            package.id,
            package.name,
            display_matches
                .iter()
                .map(|candidate| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    Ok(None)
}

fn match_by_full_addon_names(
    package: &AddonIndexPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
    expected_addon_names: &BTreeSet<String>,
) -> AppResult<Option<ExplainedTrackedPackageMatch>> {
    let full_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && tracked_package_contains_all_addons(candidate, expected_addon_names)
        })
        .cloned()
        .collect::<Vec<_>>();
    if full_matches.len() == 1 {
        return Ok(Some(ExplainedTrackedPackageMatch {
            package: full_matches[0].clone(),
            strategy: AddonIndexTrackedMatchStrategy::AddonDirectories,
        }));
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
            let overlap = tracked_package_addon_overlap(candidate, expected_addon_names);
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
        [] => Ok(None),
        [(_, candidate)] => Ok(Some(ExplainedTrackedPackageMatch {
            package: candidate.clone(),
            strategy: AddonIndexTrackedMatchStrategy::AddonDirectoryOverlap,
        })),
        [(best_overlap, best), (next_overlap, next), ..] if best_overlap > next_overlap => {
            let _ = next;
            Ok(Some(ExplainedTrackedPackageMatch {
                package: best.clone(),
                strategy: AddonIndexTrackedMatchStrategy::AddonDirectoryOverlap,
            }))
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

fn tracked_package_has_same_source_identity(
    candidate: &TrackedAddonPackage,
    source: &AddonSourceRef,
) -> bool {
    source_identity_matches(&candidate.source, source)
}

fn tracked_package_has_same_source_family_identity(
    candidate: &TrackedAddonPackage,
    source: &AddonSourceRef,
) -> bool {
    source_family_identity_matches(&candidate.source, source)
}

fn tracked_package_matches_display_name(
    candidate: &TrackedAddonPackage,
    display_name: &str,
) -> bool {
    if candidate.package_id.eq_ignore_ascii_case(display_name) {
        return true;
    }

    if candidate
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.package_name.as_deref())
        .is_some_and(|package_name| package_name.trim().eq_ignore_ascii_case(display_name))
    {
        return true;
    }

    candidate.addons.iter().any(|addon| {
        addon.directory_name.eq_ignore_ascii_case(display_name)
            || addon
                .title
                .as_deref()
                .is_some_and(|title| title.trim().eq_ignore_ascii_case(display_name))
    })
}

fn source_identity_matches(left: &AddonSourceRef, right: &AddonSourceRef) -> bool {
    match (left, right) {
        (
            AddonSourceRef::LocalArchive { path: left },
            AddonSourceRef::LocalArchive { path: right },
        ) => left == right,
        (AddonSourceRef::HttpArchive { url: left }, AddonSourceRef::HttpArchive { url: right }) => {
            left == right
        }
        (
            AddonSourceRef::CurseForgeMod {
                mod_id: left_mod_id,
                ..
            },
            AddonSourceRef::CurseForgeMod {
                mod_id: right_mod_id,
                ..
            },
        ) => left_mod_id == right_mod_id,
        (
            AddonSourceRef::GitHubRelease {
                owner: left_owner,
                repo: left_repo,
                asset_name: left_asset_name,
                ..
            },
            AddonSourceRef::GitHubRelease {
                owner: right_owner,
                repo: right_repo,
                asset_name: right_asset_name,
                ..
            },
        ) => {
            left_owner.eq_ignore_ascii_case(right_owner)
                && left_repo.eq_ignore_ascii_case(right_repo)
                && left_asset_name == right_asset_name
        }
        _ => false,
    }
}

fn source_family_identity_matches(left: &AddonSourceRef, right: &AddonSourceRef) -> bool {
    match (left, right) {
        (
            AddonSourceRef::CurseForgeMod {
                mod_id: left_mod_id,
                ..
            },
            AddonSourceRef::CurseForgeMod {
                mod_id: right_mod_id,
                ..
            },
        ) => left_mod_id == right_mod_id,
        (
            AddonSourceRef::GitHubRelease {
                owner: left_owner,
                repo: left_repo,
                ..
            },
            AddonSourceRef::GitHubRelease {
                owner: right_owner,
                repo: right_repo,
                ..
            },
        ) => {
            left_owner.eq_ignore_ascii_case(right_owner)
                && left_repo.eq_ignore_ascii_case(right_repo)
        }
        _ => false,
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

    normalize_addon_names(addon_names)
}

fn explicit_addon_names(package: &AddonIndexPackage) -> BTreeSet<String> {
    normalize_addon_names(package.addon_directories.clone())
}

fn normalized_package_ids(package_ids: &[String]) -> BTreeSet<String> {
    package_ids
        .iter()
        .map(|package_id| package_id.trim().to_ascii_lowercase())
        .filter(|package_id| !package_id.is_empty())
        .collect()
}

fn normalize_addon_names(addon_names: Vec<String>) -> BTreeSet<String> {
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
