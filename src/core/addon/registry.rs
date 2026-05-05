use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::archive_path::validate_portable_path_segment;
use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

use super::{
    AddonRegistry, AddonSourceRef, AddonStatePaths, TrackedAddonPackage, lock,
    provider::{validate_absolute_local_archive_source_path, validate_addon_source_ref},
};

pub(crate) fn load_registry(
    _installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
) -> AppResult<AddonRegistry> {
    let path = registry_path(state_paths);
    if !path.exists() {
        return Ok(AddonRegistry::default());
    }

    let content = fs::read_to_string(path)?;
    let registry = toml::from_str(&content)?;
    validate_registry(&registry)?;
    Ok(registry)
}

pub(crate) fn save_registry(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    registry: &AddonRegistry,
) -> AppResult<()> {
    let path = registry_path(state_paths);
    validate_registry(registry)?;
    if registry.packages.is_empty() {
        cleanup_registry_storage(&path)?;
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_bytes_atomically(&path, toml::to_string_pretty(registry)?.as_bytes())?;
    lock::sync_addon_lock_from_registry(installation, state_paths, registry)?;
    Ok(())
}

fn validate_registry(registry: &AddonRegistry) -> AppResult<()> {
    if registry.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon registry schema version: {}",
            registry.schema_version
        )));
    }

    let mut package_ids = BTreeMap::new();
    let mut addon_owners = BTreeMap::new();
    for package in &registry.packages {
        validate_registry_package_source(package)?;
        validate_registry_package_metadata(package)?;

        let package_id = package.package_id.trim();
        if package_id.is_empty() {
            return Err(AppError::Validation(
                "tracked addon package id cannot be empty".to_string(),
            ));
        }

        let package_key = package_id.to_ascii_lowercase();
        if let Some(existing_package_id) =
            package_ids.insert(package_key, package.package_id.clone())
        {
            return Err(AppError::Validation(format!(
                "duplicate tracked addon package id: {} conflicts with {}",
                package.package_id, existing_package_id
            )));
        }

        if package.addons.is_empty() {
            return Err(AppError::Validation(format!(
                "tracked addon package `{}` must contain at least one addon directory",
                package.package_id
            )));
        }

        let mut package_addons = BTreeMap::new();
        for addon in &package.addons {
            validate_portable_path_segment(&addon.directory_name, "addon directory").map_err(
                |error| {
                    AppError::Validation(format!(
                        "{error} for tracked package `{}`",
                        package.package_id
                    ))
                },
            )?;

            let addon_name = addon.directory_name.trim();
            let addon_key = addon_name.to_ascii_lowercase();
            if let Some(existing_addon_name) =
                package_addons.insert(addon_key.clone(), addon.directory_name.clone())
            {
                return Err(AppError::Validation(format!(
                    "duplicate addon directory `{}` in tracked package `{}` conflicts with `{}`",
                    addon.directory_name, package.package_id, existing_addon_name
                )));
            }

            if let Some((existing_package_id, existing_addon_name)) = addon_owners.insert(
                addon_key,
                (package.package_id.clone(), addon.directory_name.clone()),
            ) {
                return Err(AppError::Validation(format!(
                    "addon directory `{}` in tracked package `{}` conflicts with `{}` in tracked package `{}`",
                    addon.directory_name,
                    package.package_id,
                    existing_addon_name,
                    existing_package_id
                )));
            }
        }
    }

    Ok(())
}

fn validate_registry_package_source(package: &TrackedAddonPackage) -> AppResult<()> {
    validate_addon_source_ref(
        &package.source,
        &format!("source for tracked addon package `{}`", package.package_id),
    )?;

    match &package.source {
        AddonSourceRef::LocalArchive { path } => validate_absolute_local_archive_source_path(path)
            .map_err(|error| {
                AppError::Validation(format!(
                    "tracked addon package `{}` has an invalid local archive source: {error}",
                    package.package_id
                ))
            }),
        _ => Ok(()),
    }
}

fn validate_registry_package_metadata(package: &TrackedAddonPackage) -> AppResult<()> {
    let Some(metadata) = &package.metadata else {
        return Ok(());
    };

    for (field, value) in [
        ("index_name", metadata.index_name.as_deref()),
        ("index_package_id", metadata.index_package_id.as_deref()),
        ("package_name", metadata.package_name.as_deref()),
        ("version", metadata.version.as_deref()),
        ("source_url", metadata.source_url.as_deref()),
        ("website_url", metadata.website_url.as_deref()),
        ("source_sha256", metadata.source_sha256.as_deref()),
    ] {
        if value.is_some_and(|value| value.trim().is_empty()) {
            return Err(AppError::Validation(format!(
                "tracked addon metadata `{field}` must not be blank for package `{}`",
                package.package_id
            )));
        }
    }

    for flavor in &metadata.supported_flavors {
        if flavor.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "tracked addon metadata supported flavor must not be blank for package `{}`",
                package.package_id
            )));
        }
    }

    Ok(())
}

fn cleanup_registry_storage(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    let lock_path = path.with_file_name("lock.toml");
    if lock_path.exists() {
        fs::remove_file(lock_path)?;
    }

    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if !parent.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(parent)?;
    if entries.next().is_none() {
        fs::remove_dir(parent)?;
    }

    Ok(())
}

pub(crate) fn registry_path(state_paths: &AddonStatePaths) -> PathBuf {
    state_paths.registry_path.clone()
}

pub(crate) fn select_tracked_packages(
    registry: &AddonRegistry,
    name: Option<&str>,
) -> AppResult<Vec<TrackedAddonPackage>> {
    match name {
        None => Ok(registry.packages.clone()),
        Some(name) => {
            let name = normalize_tracked_package_selector(name)?;
            let mut matches = registry
                .packages
                .iter()
                .filter(|package| tracked_package_matches_name(package, name))
                .cloned()
                .collect::<Vec<_>>();
            matches.sort_by(|left, right| left.package_id.cmp(&right.package_id));
            if matches.is_empty() {
                return Err(AppError::NotFound(format!(
                    "no tracked addon package matched `{name}`"
                )));
            }
            Ok(matches)
        }
    }
}

pub(crate) fn select_single_tracked_package(
    registry: &AddonRegistry,
    name: &str,
) -> AppResult<(usize, TrackedAddonPackage)> {
    let name = normalize_tracked_package_selector(name)?;
    let mut matches = registry
        .packages
        .iter()
        .enumerate()
        .filter(|(_, package)| tracked_package_matches_name(package, name))
        .map(|(index, package)| (index, package.clone()))
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| left.1.package_id.cmp(&right.1.package_id));

    match matches.as_slice() {
        [] => Err(AppError::NotFound(format!(
            "no tracked addon package matched `{name}`"
        ))),
        [single] => Ok(single.clone()),
        _ => Err(AppError::Validation(format!(
            "tracked addon selector `{name}` matched multiple packages: {}",
            matches
                .iter()
                .map(|(_, package)| package.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn normalize_tracked_package_selector(name: &str) -> AppResult<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Validation(
            "tracked addon selector must not be empty".to_string(),
        ));
    }

    Ok(name)
}

fn tracked_package_matches_name(package: &TrackedAddonPackage, name: &str) -> bool {
    package.package_id.eq_ignore_ascii_case(name)
        || package
            .addons
            .iter()
            .any(|addon| addon.directory_name.eq_ignore_ascii_case(name))
}
