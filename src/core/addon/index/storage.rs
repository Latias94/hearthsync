use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::addon::{canonicalize_local_archive_path, validate_addon_source_ref};
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::{AppError, AppResult};

use super::{
    AddonIndex, AddonIndexIdentityHintCoverage, AddonIndexInspection, AddonIndexInspectionWarning,
    AddonIndexInspectionWarningCode, AddonIndexInspectionWarningSeverity, AddonIndexPackage,
    AddonSourceRef,
};

pub fn inspect_addon_index(path: &Path) -> AppResult<AddonIndexInspection> {
    let index = load_addon_index(path)?;
    let package_count = index.packages.len();
    let identity_hint_coverage = inspect_identity_hint_coverage(&index);
    let warnings = build_inspection_warnings(&identity_hint_coverage);
    let warning_count = warnings.len();
    let blocking_warning_count = warnings
        .iter()
        .filter(|warning| {
            matches!(
                warning.severity,
                AddonIndexInspectionWarningSeverity::Blocking
            )
        })
        .count();
    let advisory_warning_count = warning_count.saturating_sub(blocking_warning_count);

    Ok(AddonIndexInspection {
        index_path: path.to_path_buf(),
        index,
        package_count,
        identity_hint_coverage,
        warning_count,
        blocking_warning_count,
        advisory_warning_count,
        warnings,
    })
}

pub(super) fn write_addon_index(path: &Path, index: &AddonIndex, overwrite: bool) -> AppResult<()> {
    validate_addon_index(index)?;
    if path.exists() && !overwrite {
        return Err(AppError::Validation(format!(
            "addon index file already exists: {}. Re-run with overwrite enabled to replace it.",
            path.display()
        )));
    }

    write_bytes_atomically(path, toml::to_string_pretty(index)?.as_bytes())
}

pub(super) fn load_addon_index(path: &Path) -> AppResult<AddonIndex> {
    let content = fs::read_to_string(path)?;
    let index = toml::from_str::<AddonIndex>(&content)?;
    validate_addon_index(&index)?;
    Ok(index)
}

pub(super) fn resolve_index_package_source(
    index_path: &Path,
    source: &AddonSourceRef,
) -> AppResult<AddonSourceRef> {
    match source {
        AddonSourceRef::LocalArchive { path } => Ok(AddonSourceRef::LocalArchive {
            path: resolve_index_local_archive_path(index_path, path)?,
        }),
        other => Ok(other.clone()),
    }
}

fn resolve_index_local_archive_path(index_path: &Path, path: &Path) -> AppResult<PathBuf> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        index_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
    };

    canonicalize_local_archive_path(&candidate).map_err(|error| match error {
        AppError::NotFound(_) => AppError::NotFound(format!(
            "addon index local archive source does not exist: {}",
            candidate.display()
        )),
        AppError::Validation(_) => AppError::Validation(format!(
            "addon index local archive source must be a file archive: {}",
            candidate.display()
        )),
        other => other,
    })
}

fn validate_addon_index(index: &AddonIndex) -> AppResult<()> {
    if index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon index schema version: {}",
            index.schema_version
        )));
    }
    if index.name.trim().is_empty() {
        return Err(AppError::Validation(
            "addon index name must not be empty".to_string(),
        ));
    }
    if index.packages.is_empty() {
        return Err(AppError::Validation(
            "addon index must contain at least one package".to_string(),
        ));
    }

    let mut ids = BTreeSet::new();
    for package in &index.packages {
        validate_index_package(package)?;
        let normalized_id = package.id.trim().to_ascii_lowercase();
        if !ids.insert(normalized_id) {
            return Err(AppError::Validation(format!(
                "duplicate addon index package id: {}",
                package.id
            )));
        }
    }

    Ok(())
}

fn validate_index_package(package: &AddonIndexPackage) -> AppResult<()> {
    for (field, value) in [
        ("package id", &package.id),
        ("package name", &package.name),
        ("package version", &package.version),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::Validation(format!("{field} must not be empty")));
        }
    }

    validate_addon_source_ref(
        &package.source,
        &format!("source for package `{}`", package.id),
    )?;
    validate_optional_package_text(package, "source_url", package.source_url.as_deref())?;
    validate_optional_package_text(package, "website_url", package.website_url.as_deref())?;
    validate_optional_package_text(package, "sha256", package.sha256.as_deref())?;

    let mut normalized_match_package_ids = BTreeSet::new();
    for match_package_id in &package.match_package_ids {
        let normalized = match_package_id.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(AppError::Validation(format!(
                "match package id must not be empty for package `{}`",
                package.id
            )));
        }
        if !normalized_match_package_ids.insert(normalized) {
            return Err(AppError::Validation(format!(
                "duplicate match package id for package `{}`",
                package.id
            )));
        }
    }

    let mut normalized_addon_directories = BTreeSet::new();
    for addon_directory in &package.addon_directories {
        validate_portable_path_segment(addon_directory, "addon directory").map_err(|error| {
            match error {
                AppError::Validation(message) => {
                    AppError::Validation(format!("{message} for package `{}`", package.id))
                }
                other => other,
            }
        })?;

        let normalized = addon_directory.trim().to_ascii_lowercase();
        if !normalized_addon_directories.insert(normalized) {
            return Err(AppError::Validation(format!(
                "duplicate addon directory for package `{}`",
                package.id
            )));
        }
    }

    for flavor in &package.supported_flavors {
        if flavor.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "supported flavor must not be empty for package `{}`",
                package.id
            )));
        }
    }

    Ok(())
}

fn validate_optional_package_text(
    package: &AddonIndexPackage,
    field: &str,
    value: Option<&str>,
) -> AppResult<()> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        return Err(AppError::Validation(format!(
            "`{field}` must not be blank for package `{}`",
            package.id
        )));
    }

    Ok(())
}

pub(super) fn find_index_package<'a>(
    index: &'a AddonIndex,
    name: &str,
) -> AppResult<&'a AddonIndexPackage> {
    index
        .packages
        .iter()
        .find(|package| {
            package.id.eq_ignore_ascii_case(name) || package.name.eq_ignore_ascii_case(name)
        })
        .ok_or_else(|| AppError::NotFound(format!("addon index package `{name}` not found")))
}

pub(super) fn ensure_package_supports_flavor(
    package: &AddonIndexPackage,
    flavor: &str,
) -> AppResult<()> {
    if package.supported_flavors.is_empty()
        || package
            .supported_flavors
            .iter()
            .any(|item| item.eq_ignore_ascii_case(flavor))
    {
        return Ok(());
    }

    Err(AppError::Validation(format!(
        "package `{}` does not support flavor `{}`. Supported flavors: {}",
        package.id,
        flavor,
        package.supported_flavors.join(", ")
    )))
}

fn inspect_identity_hint_coverage(index: &AddonIndex) -> AddonIndexIdentityHintCoverage {
    let mut package_count_with_both_exact_hints = 0;
    let mut package_count_with_match_package_ids = 0;
    let mut package_count_with_addon_directories = 0;
    let mut package_count_with_any_exact_hints = 0;
    let mut packages_without_match_package_ids = Vec::new();
    let mut packages_without_addon_directories = Vec::new();
    let mut packages_without_exact_hints = Vec::new();

    for package in &index.packages {
        let has_match_package_ids = !package.match_package_ids.is_empty();
        let has_addon_directories = !package.addon_directories.is_empty();

        if has_match_package_ids && has_addon_directories {
            package_count_with_both_exact_hints += 1;
        }
        if has_match_package_ids {
            package_count_with_match_package_ids += 1;
        } else {
            packages_without_match_package_ids.push(package.id.clone());
        }
        if has_addon_directories {
            package_count_with_addon_directories += 1;
        } else {
            packages_without_addon_directories.push(package.id.clone());
        }
        if has_match_package_ids || has_addon_directories {
            package_count_with_any_exact_hints += 1;
        } else {
            packages_without_exact_hints.push(package.id.clone());
        }
    }

    AddonIndexIdentityHintCoverage {
        package_count_with_both_exact_hints,
        package_count_with_any_exact_hints,
        package_count_with_match_package_ids,
        package_count_with_addon_directories,
        package_count_without_match_package_ids: packages_without_match_package_ids.len(),
        package_count_without_addon_directories: packages_without_addon_directories.len(),
        package_count_without_exact_hints: packages_without_exact_hints.len(),
        packages_without_match_package_ids,
        packages_without_addon_directories,
        packages_without_exact_hints,
    }
}

fn build_inspection_warnings(
    coverage: &AddonIndexIdentityHintCoverage,
) -> Vec<AddonIndexInspectionWarning> {
    let packages_without_exact_hints = coverage
        .packages_without_exact_hints
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut warnings = Vec::new();

    for package_id in &coverage.packages_without_match_package_ids {
        if packages_without_exact_hints.contains(package_id) {
            continue;
        }

        warnings.push(AddonIndexInspectionWarning {
            code: AddonIndexInspectionWarningCode::MissingMatchPackageIds,
            severity: AddonIndexInspectionWarningSeverity::Advisory,
            package_id: package_id.clone(),
            message: format!(
                "package `{package_id}` declares `addon_directories` but not `match_package_ids`; add curated historical package ids when known so addon-index preflight can preserve package-id continuity across source-family drift"
            ),
        });
    }

    for package_id in &coverage.packages_without_addon_directories {
        if packages_without_exact_hints.contains(package_id) {
            continue;
        }

        warnings.push(AddonIndexInspectionWarning {
            code: AddonIndexInspectionWarningCode::MissingAddonDirectories,
            severity: AddonIndexInspectionWarningSeverity::Advisory,
            package_id: package_id.clone(),
            message: format!(
                "package `{package_id}` declares `match_package_ids` but not explicit `addon_directories`; add stable addon directory names so addon-index preflight can preserve directory continuity without downloading package contents first"
            ),
        });
    }

    for package_id in &coverage.packages_without_exact_hints {
        warnings.push(AddonIndexInspectionWarning {
            code: AddonIndexInspectionWarningCode::MissingExactIdentityHints,
            severity: AddonIndexInspectionWarningSeverity::Blocking,
            package_id: package_id.clone(),
            message: format!(
                "package `{package_id}` does not declare exact identity hints (`match_package_ids` or `addon_directories`), so addon-index preflight may need to fall back to domain matching if source identity drifts"
            ),
        });
    }

    warnings
}
