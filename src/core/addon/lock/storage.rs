use std::fs::File;
use std::io::Read;

use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::core::addon::{
    AddonPackageMetadata, AddonRegistry, AddonSourceRef, AddonStatePaths, TrackedAddon,
    TrackedAddonPackage, find_existing_addon_path, load_registry, validate_addon_source_ref,
};
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

use super::{
    AddonLock, AddonLockInspection, AddonLockPackage, AddonLockWriteResult, comparison_key,
};

pub fn inspect_addon_lock(
    _installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
) -> AppResult<AddonLockInspection> {
    let path = lock_path(state_paths);
    let lock = read_addon_lock(&path)?;
    let package_count = lock.packages.len();

    Ok(AddonLockInspection {
        lock_path: path,
        lock,
        package_count,
    })
}

pub fn write_addon_lock(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
) -> AppResult<AddonLockWriteResult> {
    let registry = load_registry(installation, state_paths)?;
    let path = lock_path(state_paths);
    if registry.packages.is_empty() {
        cleanup_addon_lock(&path)?;
        return Ok(AddonLockWriteResult {
            lock_path: path,
            package_count: 0,
            removed: true,
        });
    }

    let lock = build_addon_lock(installation, &registry)?;
    write_addon_lock_file(&path, &lock)?;

    Ok(AddonLockWriteResult {
        lock_path: path,
        package_count: lock.packages.len(),
        removed: false,
    })
}

pub(crate) fn sync_addon_lock_from_registry(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    registry: &AddonRegistry,
) -> AppResult<()> {
    let path = lock_path(state_paths);
    if registry.packages.is_empty() {
        cleanup_addon_lock(&path)?;
        return Ok(());
    }

    let lock = build_addon_lock(installation, registry)?;
    write_addon_lock_file(&path, &lock)
}

pub fn lock_path(state_paths: &AddonStatePaths) -> PathBuf {
    state_paths.lock_path.clone()
}

fn build_addon_lock(
    installation: &DetectedFlavorInstallation,
    registry: &AddonRegistry,
) -> AppResult<AddonLock> {
    let mut packages = registry
        .packages
        .iter()
        .map(|package| build_lock_package(installation, package))
        .collect::<AppResult<Vec<_>>>()?;
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));

    Ok(AddonLock {
        schema_version: 1,
        generated_at: now_rfc3339()?,
        packages,
    })
}

fn build_lock_package(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<AddonLockPackage> {
    let metadata = package.metadata.as_ref();
    let mut addons = package.addons.clone();
    addons.sort_by(|left, right| left.directory_name.cmp(&right.directory_name));
    let addon_directories = addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>();

    Ok(AddonLockPackage {
        package_id: package.package_id.clone(),
        index_name: metadata.and_then(|value| value.index_name.clone()),
        index_package_id: metadata.and_then(|value| value.index_package_id.clone()),
        name: lock_package_name(package, metadata),
        version: lock_package_version(package, metadata),
        source: package.source.clone(),
        source_url: metadata.and_then(|value| value.source_url.clone()),
        website_url: metadata.and_then(|value| value.website_url.clone()),
        source_sha256: metadata.and_then(|value| value.source_sha256.clone()),
        content_sha256: package_content_sha256(installation, package)?,
        installed_at: package.installed_at.clone(),
        updated_at: package.updated_at.clone(),
        addon_directories,
        addons,
    })
}

pub(crate) fn read_addon_lock(path: &Path) -> AppResult<AddonLock> {
    let content = fs::read_to_string(path)?;
    let lock = toml::from_str::<AddonLock>(&content)?;
    validate_addon_lock(&lock)?;
    Ok(lock)
}

pub(super) fn lock_package_name(
    package: &TrackedAddonPackage,
    metadata: Option<&AddonPackageMetadata>,
) -> Option<String> {
    metadata
        .and_then(|value| value.package_name.clone())
        .or_else(|| {
            package
                .addons
                .iter()
                .filter_map(|addon| addon.title.clone())
                .find(|value| !value.trim().is_empty())
        })
        .or_else(|| Some(package.package_id.clone()))
}

pub(super) fn lock_package_version(
    package: &TrackedAddonPackage,
    metadata: Option<&AddonPackageMetadata>,
) -> Option<String> {
    metadata
        .and_then(|value| value.version.clone())
        .or_else(|| infer_addon_version(&package.addons))
}

fn infer_addon_version(addons: &[TrackedAddon]) -> Option<String> {
    let versions = addons
        .iter()
        .filter_map(|addon| addon.version.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>();
    if versions.len() == 1 {
        versions.iter().next().map(|value| (*value).to_string())
    } else {
        None
    }
}

fn package_content_sha256(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<String> {
    let (content_sha256, missing_addon_directories) =
        package_content_sha256_with_missing(installation, package)?;
    if !missing_addon_directories.is_empty() {
        return Err(AppError::NotFound(format!(
            "tracked addon directories missing for package `{}`: {}",
            package.package_id,
            missing_addon_directories.join(", ")
        )));
    }
    Ok(content_sha256.unwrap_or_default())
}

pub(super) fn package_content_sha256_with_missing(
    installation: &DetectedFlavorInstallation,
    package: &TrackedAddonPackage,
) -> AppResult<(Option<String>, Vec<String>)> {
    let mut files = Vec::new();
    let mut missing_addon_directories = Vec::new();
    for addon in &package.addons {
        let Some(existing) = find_existing_addon_path(
            &installation.addon_dir,
            &addon.directory_name,
            installation.platform,
        )?
        else {
            missing_addon_directories.push(addon.directory_name.clone());
            continue;
        };
        let addon_path = existing.path;

        for entry in WalkDir::new(&addon_path) {
            let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
            if !entry.file_type().is_file() {
                continue;
            }
            let relative_path = normalize_relative_path(entry.path(), &installation.addon_dir)?;
            files.push((relative_path, entry.path().to_path_buf()));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    if !missing_addon_directories.is_empty() {
        return Ok((None, missing_addon_directories));
    }

    let mut hasher = Sha256::new();
    for (relative_path, path) in files {
        hash_file_entry(&mut hasher, &relative_path, &path)?;
    }

    Ok((
        Some(format!("{:x}", hasher.finalize())),
        missing_addon_directories,
    ))
}

fn hash_file_entry(hasher: &mut Sha256, relative_path: &str, path: &Path) -> AppResult<()> {
    let length = fs::metadata(path)?.len();
    hasher.update(relative_path.as_bytes());
    hasher.update([0]);
    hasher.update(length.to_le_bytes());
    hasher.update([0]);

    let mut file = File::open(path)?;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    hasher.update([0]);

    Ok(())
}

fn normalize_relative_path(path: &Path, base: &Path) -> AppResult<String> {
    let relative = path.strip_prefix(base).map_err(|_| {
        AppError::Validation(format!(
            "path `{}` is outside addon root `{}`",
            path.display(),
            base.display()
        ))
    })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn write_addon_lock_file(path: &Path, lock: &AddonLock) -> AppResult<()> {
    validate_addon_lock(lock)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_bytes_atomically(path, toml::to_string_pretty(lock)?.as_bytes())?;
    Ok(())
}

fn cleanup_addon_lock(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
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

fn validate_addon_lock(lock: &AddonLock) -> AppResult<()> {
    if lock.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon lock schema version: {}",
            lock.schema_version
        )));
    }
    if lock.generated_at.trim().is_empty() {
        return Err(AppError::Validation(
            "addon lock generated_at must not be empty".to_string(),
        ));
    }

    let mut comparison_keys = BTreeSet::new();
    let mut package_ids = BTreeSet::new();
    let mut addon_owners = BTreeMap::new();
    for package in &lock.packages {
        validate_addon_lock_package(package, &mut addon_owners)?;

        let package_id_key = package.package_id.trim().to_ascii_lowercase();
        if !package_ids.insert(package_id_key) {
            return Err(AppError::Validation(format!(
                "duplicate addon lock package id: {}",
                package.package_id
            )));
        }

        let comparison_key = comparison_key(
            &package.package_id,
            package.index_name.as_deref(),
            package.index_package_id.as_deref(),
            &package.addon_directories,
        );
        if !comparison_keys.insert(comparison_key.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate addon lock package comparison key: {comparison_key}"
            )));
        }
    }
    Ok(())
}

fn validate_addon_lock_package(
    package: &AddonLockPackage,
    addon_owners: &mut BTreeMap<String, String>,
) -> AppResult<()> {
    if package.package_id.trim().is_empty() {
        return Err(AppError::Validation(
            "addon lock package id must not be empty".to_string(),
        ));
    }

    for (field, value) in [
        ("index_name", package.index_name.as_deref()),
        ("index_package_id", package.index_package_id.as_deref()),
        ("name", package.name.as_deref()),
        ("version", package.version.as_deref()),
        ("source_url", package.source_url.as_deref()),
        ("website_url", package.website_url.as_deref()),
        ("source_sha256", package.source_sha256.as_deref()),
    ] {
        validate_optional_lock_text(package, field, value)?;
    }

    validate_addon_source_ref(
        &package.source,
        &format!("source for addon lock package `{}`", package.package_id),
    )?;
    validate_lock_local_archive_source(package)?;

    validate_required_lock_text(package, "content_sha256", &package.content_sha256)?;
    if !is_sha256_hex(&package.content_sha256) {
        return Err(AppError::Validation(format!(
            "addon lock package `{}` content_sha256 must be a 64-character SHA-256 hex digest",
            package.package_id
        )));
    }
    validate_required_lock_text(package, "installed_at", &package.installed_at)?;
    validate_required_lock_text(package, "updated_at", &package.updated_at)?;

    if package.addon_directories.is_empty() {
        return Err(AppError::Validation(format!(
            "addon lock package `{}` must contain at least one addon directory",
            package.package_id
        )));
    }

    let mut package_addons = BTreeSet::new();
    for addon_directory in &package.addon_directories {
        validate_lock_addon_directory(package, addon_directory)?;
        let addon_key = addon_directory.trim().to_ascii_lowercase();
        if !package_addons.insert(addon_key.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate addon directory `{}` in addon lock package `{}`",
                addon_directory, package.package_id
            )));
        }
        if let Some(existing_package_id) =
            addon_owners.insert(addon_key, package.package_id.clone())
        {
            return Err(AppError::Validation(format!(
                "addon directory `{}` in addon lock package `{}` conflicts with addon lock package `{}`",
                addon_directory, package.package_id, existing_package_id
            )));
        }
    }

    for addon in &package.addons {
        validate_lock_addon_directory(package, &addon.directory_name)?;
        validate_optional_lock_text(package, "addon.toc_file", addon.toc_file.as_deref())?;
        if let Some(toc_file) = &addon.toc_file {
            validate_portable_path_segment(toc_file, "addon toc file").map_err(|error| {
                AppError::Validation(format!(
                    "{error} for addon lock package `{}`",
                    package.package_id
                ))
            })?;
        }
        validate_optional_lock_text(package, "addon.title", addon.title.as_deref())?;
        validate_optional_lock_text(package, "addon.version", addon.version.as_deref())?;
    }

    Ok(())
}

fn validate_lock_local_archive_source(package: &AddonLockPackage) -> AppResult<()> {
    let AddonSourceRef::LocalArchive { path } = &package.source else {
        return Ok(());
    };

    if path.is_absolute() {
        return Ok(());
    }

    Err(AppError::Validation(format!(
        "invalid source for addon lock package `{}`: local archive source path must be absolute before lock planning: {}",
        package.package_id,
        path.display()
    )))
}

fn validate_lock_addon_directory(
    package: &AddonLockPackage,
    addon_directory: &str,
) -> AppResult<()> {
    validate_portable_path_segment(addon_directory, "addon directory").map_err(|error| {
        AppError::Validation(format!(
            "{error} for addon lock package `{}`",
            package.package_id
        ))
    })
}

fn validate_required_lock_text(
    package: &AddonLockPackage,
    field: &str,
    value: &str,
) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "addon lock package `{}` {field} must not be empty",
            package.package_id
        )));
    }

    Ok(())
}

fn validate_optional_lock_text(
    package: &AddonLockPackage,
    field: &str,
    value: Option<&str>,
) -> AppResult<()> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        return Err(AppError::Validation(format!(
            "addon lock package `{}` {field} must not be blank",
            package.package_id
        )));
    }

    Ok(())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.chars().all(|char| char.is_ascii_hexdigit())
}

pub(super) fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}
