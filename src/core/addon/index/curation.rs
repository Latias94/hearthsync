use std::collections::BTreeSet;

use crate::core::addon::{
    TrackedAddonPackage, list_addons, load_registry, no_tracked_packages_error,
    select_tracked_packages,
};
use crate::core::error::{AppError, AppResult};

use super::matching::{
    explain_preflight_match_index_package_to_tracked_package, package_id_usage_key,
};
use super::storage::{
    ensure_package_supports_flavor, load_addon_index, resolve_index_package_source,
    write_addon_index,
};
use super::{
    AddonIndex, AddonIndexPackage, AddonIndexPackageSuggestion, AddonIndexPackageSuggestionStatus,
    AddonIndexScaffoldRequest, AddonIndexScaffoldResult, AddonIndexSuggestion,
    AddonIndexSuggestionRequest, AddonIndexTrackedMatchStrategy,
};

pub fn suggest_addon_index_hints(
    request: AddonIndexSuggestionRequest,
) -> AppResult<AddonIndexSuggestion> {
    let index = load_addon_index(&request.index_path)?;
    let index_package_count = index.packages.len();
    let selected_packages = select_packages_for_suggestion(&index.packages, &request)?;
    let skipped_unsupported_flavor_package_count =
        index_package_count.saturating_sub(selected_packages.len());
    let inventory = list_addons(&request.installation, &request.state_paths)?;
    let mut used_package_ids = BTreeSet::new();
    let mut packages = Vec::new();

    for package in selected_packages {
        let package_for_matching =
            resolved_index_package_for_matching(&request.index_path, package);
        let suggestion = match explain_preflight_match_index_package_to_tracked_package(
            &package_for_matching,
            &inventory.tracked_packages,
            &used_package_ids,
        ) {
            Ok(Some(matched)) => {
                let matched_package_id = matched.package.package_id.clone();
                used_package_ids.insert(package_id_usage_key(&matched_package_id));
                matched_package_suggestion(package, matched.package, matched.strategy)
            }
            Ok(None) => no_local_match_suggestion(package),
            Err(error) => ambiguous_local_match_suggestion(package, error.to_string()),
        };
        packages.push(suggestion);
    }

    let suggested_package_count = packages
        .iter()
        .filter(|package| matches!(package.status, AddonIndexPackageSuggestionStatus::Suggested))
        .count();
    let complete_package_count = packages
        .iter()
        .filter(|package| matches!(package.status, AddonIndexPackageSuggestionStatus::Complete))
        .count();
    let no_match_package_count = packages
        .iter()
        .filter(|package| {
            matches!(
                package.status,
                AddonIndexPackageSuggestionStatus::NoLocalMatch
            )
        })
        .count();
    let ambiguous_match_package_count = packages
        .iter()
        .filter(|package| {
            matches!(
                package.status,
                AddonIndexPackageSuggestionStatus::AmbiguousLocalMatch
            )
        })
        .count();

    Ok(AddonIndexSuggestion {
        index_path: request.index_path,
        index_name: index.name,
        index_package_count,
        considered_package_count: packages.len(),
        suggested_package_count,
        complete_package_count,
        no_match_package_count,
        ambiguous_match_package_count,
        skipped_unsupported_flavor_package_count,
        packages,
    })
}

pub fn scaffold_addon_index(
    request: AddonIndexScaffoldRequest,
) -> AppResult<AddonIndexScaffoldResult> {
    let registry = load_registry(&request.installation, &request.state_paths)?;
    let selected_packages = select_tracked_packages(&registry, request.name.as_deref())?;
    if selected_packages.is_empty() {
        return Err(no_tracked_packages_error(
            &request.installation,
            &request.state_paths,
        ));
    }

    let mut scaffolded_packages = selected_packages
        .iter()
        .map(|package| scaffold_index_package(package, request.installation.flavor.as_str()))
        .collect::<Vec<_>>();
    scaffolded_packages.sort_by(|left, right| left.package.id.cmp(&right.package.id));

    let package_count = scaffolded_packages.len();
    let used_metadata_package_count = scaffolded_packages
        .iter()
        .filter(|package| package.used_metadata)
        .count();
    let inferred_name_package_count = scaffolded_packages
        .iter()
        .filter(|package| package.inferred_name)
        .count();
    let inferred_version_package_count = scaffolded_packages
        .iter()
        .filter(|package| package.inferred_version)
        .count();
    let placeholder_version_package_count = scaffolded_packages
        .iter()
        .filter(|package| package.placeholder_version)
        .count();
    let package_ids = scaffolded_packages
        .iter()
        .map(|package| package.package.id.clone())
        .collect::<Vec<_>>();

    let index = AddonIndex {
        schema_version: 1,
        name: request.index_name.clone(),
        description: request.description.clone(),
        packages: scaffolded_packages
            .into_iter()
            .map(|package| package.package)
            .collect(),
    };
    write_addon_index(&request.index_path, &index, request.overwrite)?;

    Ok(AddonIndexScaffoldResult {
        index_path: request.index_path,
        index_name: request.index_name,
        package_count,
        used_metadata_package_count,
        inferred_name_package_count,
        inferred_version_package_count,
        placeholder_version_package_count,
        package_ids,
    })
}

fn select_packages_for_suggestion<'a>(
    packages: &'a [AddonIndexPackage],
    request: &AddonIndexSuggestionRequest,
) -> AppResult<Vec<&'a AddonIndexPackage>> {
    match request.name.as_deref() {
        Some(name) => {
            let package = find_index_package_in_slice(packages, name)?;
            ensure_package_supports_flavor(package, request.installation.flavor.as_str())?;
            Ok(vec![package])
        }
        None => Ok(packages
            .iter()
            .filter(|package| supports_flavor(package, request.installation.flavor.as_str()))
            .collect()),
    }
}

fn resolved_index_package_for_matching(
    index_path: &std::path::Path,
    package: &AddonIndexPackage,
) -> AddonIndexPackage {
    let mut resolved = package.clone();
    if let Ok(source) = resolve_index_package_source(index_path, &package.source) {
        resolved.source = source;
    }
    resolved
}

fn find_index_package_in_slice<'a>(
    packages: &'a [AddonIndexPackage],
    name: &str,
) -> AppResult<&'a AddonIndexPackage> {
    packages
        .iter()
        .find(|package| {
            package.id.eq_ignore_ascii_case(name) || package.name.eq_ignore_ascii_case(name)
        })
        .ok_or_else(|| AppError::NotFound(format!("addon index package `{name}` not found")))
}

fn supports_flavor(package: &AddonIndexPackage, flavor: &str) -> bool {
    package.supported_flavors.is_empty()
        || package
            .supported_flavors
            .iter()
            .any(|item| item.eq_ignore_ascii_case(flavor))
}

fn matched_package_suggestion(
    package: &AddonIndexPackage,
    matched: crate::core::addon::TrackedAddonPackage,
    strategy: AddonIndexTrackedMatchStrategy,
) -> AddonIndexPackageSuggestion {
    let match_package_ids_to_add = missing_match_package_ids(package, &matched.package_id);
    let matched_addon_directories = matched
        .addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>();
    let addon_directories_to_add =
        missing_addon_directories(package, matched_addon_directories.as_slice());
    let (status, message) = if match_package_ids_to_add.is_empty()
        && addon_directories_to_add.is_empty()
    {
        (
            AddonIndexPackageSuggestionStatus::Complete,
            format!(
                "matched tracked package `{}` by {}; current index hints already cover this local package",
                matched.package_id,
                match_strategy_label(&strategy)
            ),
        )
    } else {
        (
            AddonIndexPackageSuggestionStatus::Suggested,
            format!(
                "matched tracked package `{}` by {}; add missing exact identity hints from the current local registry",
                matched.package_id,
                match_strategy_label(&strategy)
            ),
        )
    };

    AddonIndexPackageSuggestion {
        package_id: package.id.clone(),
        package_name: package.name.clone(),
        current_match_package_ids: package.match_package_ids.clone(),
        current_addon_directories: package.addon_directories.clone(),
        status,
        matched_tracked_package_id: Some(matched.package_id),
        match_strategy: Some(strategy),
        matched_addon_directories,
        match_package_ids_to_add,
        addon_directories_to_add,
        message,
    }
}

#[derive(Debug, Clone)]
struct ScaffoldedIndexPackage {
    package: AddonIndexPackage,
    used_metadata: bool,
    inferred_name: bool,
    inferred_version: bool,
    placeholder_version: bool,
}

fn scaffold_index_package(
    tracked: &TrackedAddonPackage,
    installation_flavor: &str,
) -> ScaffoldedIndexPackage {
    let metadata = tracked.metadata.as_ref();
    let used_metadata = metadata.is_some();
    let package_id = metadata
        .and_then(|metadata| metadata.index_package_id.clone())
        .unwrap_or_else(|| tracked.package_id.clone());
    let inferred_name = metadata
        .and_then(|metadata| metadata.package_name.as_deref())
        .is_none();
    let package_name = metadata
        .and_then(|metadata| metadata.package_name.clone())
        .or_else(|| {
            tracked
                .addons
                .iter()
                .find_map(|addon| addon.title.as_ref().map(|title| title.trim().to_string()))
        })
        .filter(|name| !name.is_empty())
        .or_else(|| {
            tracked
                .addons
                .first()
                .map(|addon| addon.directory_name.clone())
        })
        .unwrap_or_else(|| tracked.package_id.clone());
    let version_from_metadata = metadata.and_then(|metadata| metadata.version.clone());
    let version_from_addons = tracked
        .addons
        .iter()
        .find_map(|addon| {
            addon
                .version
                .as_ref()
                .map(|version| version.trim().to_string())
        })
        .filter(|version| !version.is_empty());
    let inferred_version = version_from_metadata.is_none();
    let version = version_from_metadata
        .or(version_from_addons)
        .unwrap_or_else(|| "unknown".to_string());
    let placeholder_version = version.eq_ignore_ascii_case("unknown");
    let addon_directories = tracked
        .addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>();
    let supported_flavors = metadata
        .map(|metadata| metadata.supported_flavors.clone())
        .filter(|flavors| !flavors.is_empty())
        .unwrap_or_else(|| vec![installation_flavor.to_string()]);
    let match_package_ids = if package_id.eq_ignore_ascii_case(&tracked.package_id) {
        Vec::new()
    } else {
        vec![tracked.package_id.clone()]
    };

    ScaffoldedIndexPackage {
        package: AddonIndexPackage {
            id: package_id,
            name: package_name,
            version,
            match_package_ids,
            source: tracked.source.clone(),
            source_url: metadata.and_then(|metadata| metadata.source_url.clone()),
            website_url: metadata.and_then(|metadata| metadata.website_url.clone()),
            sha256: metadata.and_then(|metadata| metadata.source_sha256.clone()),
            addon_directories,
            supported_flavors,
        },
        used_metadata,
        inferred_name,
        inferred_version,
        placeholder_version,
    }
}

fn no_local_match_suggestion(package: &AddonIndexPackage) -> AddonIndexPackageSuggestion {
    AddonIndexPackageSuggestion {
        package_id: package.id.clone(),
        package_name: package.name.clone(),
        current_match_package_ids: package.match_package_ids.clone(),
        current_addon_directories: package.addon_directories.clone(),
        status: AddonIndexPackageSuggestionStatus::NoLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        matched_addon_directories: Vec::new(),
        match_package_ids_to_add: Vec::new(),
        addon_directories_to_add: Vec::new(),
        message: "no tracked addon package from the current registry matched this index package"
            .to_string(),
    }
}

fn ambiguous_local_match_suggestion(
    package: &AddonIndexPackage,
    message: String,
) -> AddonIndexPackageSuggestion {
    AddonIndexPackageSuggestion {
        package_id: package.id.clone(),
        package_name: package.name.clone(),
        current_match_package_ids: package.match_package_ids.clone(),
        current_addon_directories: package.addon_directories.clone(),
        status: AddonIndexPackageSuggestionStatus::AmbiguousLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        matched_addon_directories: Vec::new(),
        match_package_ids_to_add: Vec::new(),
        addon_directories_to_add: Vec::new(),
        message,
    }
}

fn missing_match_package_ids(package: &AddonIndexPackage, tracked_package_id: &str) -> Vec<String> {
    if package.id.eq_ignore_ascii_case(tracked_package_id) {
        return Vec::new();
    }

    let existing = package
        .match_package_ids
        .iter()
        .map(|item| normalize(item))
        .collect::<BTreeSet<_>>();
    if existing.contains(&normalize(tracked_package_id)) {
        return Vec::new();
    }

    vec![tracked_package_id.to_string()]
}

fn missing_addon_directories(
    package: &AddonIndexPackage,
    tracked_addon_directories: &[String],
) -> Vec<String> {
    let existing = package
        .addon_directories
        .iter()
        .map(|item| normalize(item))
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut missing = Vec::new();

    for directory in tracked_addon_directories {
        let normalized = normalize(directory);
        if existing.contains(&normalized) || !seen.insert(normalized) {
            continue;
        }
        missing.push(directory.clone());
    }

    missing
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn match_strategy_label(strategy: &AddonIndexTrackedMatchStrategy) -> &'static str {
    match strategy {
        AddonIndexTrackedMatchStrategy::StoredIndexPackageId => "stored index package id",
        AddonIndexTrackedMatchStrategy::ExactPackageId => "exact package id",
        AddonIndexTrackedMatchStrategy::CuratedMatchPackageId => "curated match_package_ids hint",
        AddonIndexTrackedMatchStrategy::SourceIdentity => "source identity",
        AddonIndexTrackedMatchStrategy::SourceFamilyIdentity => "source family identity",
        AddonIndexTrackedMatchStrategy::DisplayName => "display name",
        AddonIndexTrackedMatchStrategy::AddonDirectories => "addon directories",
        AddonIndexTrackedMatchStrategy::AddonDirectoryOverlap => "addon directory overlap",
    }
}
