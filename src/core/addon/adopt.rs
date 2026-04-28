use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use tempfile::NamedTempFile;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::core::archive_io::{
    PortableArchivePathSet, add_directory_to_zip, portable_archive_path_issue_error,
    reject_unsupported_symlink_metadata, stream_file_to_zip,
};
use crate::core::archive_path::to_zip_path;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

use super::package_prep::{inspect_addon_directory, slugify_package_id};
use super::provider::{AddonSourceRef, canonicalize_local_archive_path};
use super::{
    AddonInventory, AddonRegistry, AdoptAddonsRequest, AdoptedAddonPackageResult, TrackedAddon,
    TrackedAddonPackage,
};
use super::{load_registry, registry::registry_path, save_registry};

pub fn adopt_addons(request: AdoptAddonsRequest) -> AppResult<AdoptedAddonPackageResult> {
    if request.addon_directories.is_empty() {
        return Err(AppError::Validation(
            "at least one explicit addon directory is required for adoption".to_string(),
        ));
    }

    let inventory = super::list_addons(&request.installation, &request.state_paths)?;
    let selected_addons = resolve_requested_addons(&inventory, &request.addon_directories)?;
    let package_id = resolve_package_id(request.package_id.as_deref(), &selected_addons)?;
    let mut registry = load_registry(&request.installation, &request.state_paths)?;
    ensure_package_id_is_available(&registry, &package_id)?;

    let archive_path = resolve_snapshot_archive_path(
        &request.state_paths,
        request.archive_output_path,
        &package_id,
    )?;
    ensure_archive_path_is_available(&archive_path)?;
    ensure_archive_path_is_outside_selected_addons(
        &request.installation,
        &archive_path,
        &selected_addons,
    )?;

    let addons = inspect_selected_addons(&request.installation, &selected_addons)?;
    let registry_path = registry_path(&request.state_paths);

    if request.dry_run {
        return Ok(AdoptedAddonPackageResult {
            dry_run: true,
            source: planned_source_ref(&archive_path),
            package_id,
            addons,
            registry_path,
        });
    }

    write_snapshot_archive(&request.installation, &selected_addons, &archive_path)?;
    let source = AddonSourceRef::LocalArchive {
        path: canonicalize_local_archive_path(&archive_path)?,
    };
    let timestamp = now_rfc3339()?;
    registry.packages.push(TrackedAddonPackage {
        package_id: package_id.clone(),
        source: source.clone(),
        installed_at: timestamp.clone(),
        updated_at: timestamp,
        addons: addons.clone(),
        metadata: None,
    });
    save_registry(&request.installation, &request.state_paths, &registry)?;

    Ok(AdoptedAddonPackageResult {
        dry_run: false,
        source,
        package_id,
        addons,
        registry_path,
    })
}

fn resolve_requested_addons(
    inventory: &AddonInventory,
    requested: &[String],
) -> AppResult<Vec<String>> {
    let mut untracked_by_key = BTreeMap::new();
    for addon in &inventory.untracked_addons {
        untracked_by_key.insert(addon.trim().to_ascii_lowercase(), addon.clone());
    }

    let tracked_keys = inventory
        .tracked_packages
        .iter()
        .flat_map(|package| package.addons.iter())
        .map(|addon| addon.directory_name.trim().to_ascii_lowercase())
        .collect::<BTreeSet<_>>();

    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();
    for requested_name in requested {
        let normalized = requested_name.trim().to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(AppError::Validation(
                "addon directory names for adoption must not be empty".to_string(),
            ));
        }
        if !seen.insert(normalized.clone()) {
            return Err(AppError::Validation(format!(
                "duplicate addon directory requested for adoption: `{requested_name}`"
            )));
        }

        if let Some(actual_name) = untracked_by_key.get(&normalized) {
            selected.push(actual_name.clone());
            continue;
        }

        if tracked_keys.contains(&normalized) {
            return Err(AppError::Validation(format!(
                "addon directory `{requested_name}` is already tracked and cannot be adopted again"
            )));
        }

        return Err(AppError::NotFound(format!(
            "untracked addon directory `{requested_name}` was not found in the current installation"
        )));
    }

    Ok(selected)
}

fn resolve_package_id(package_id: Option<&str>, addon_directories: &[String]) -> AppResult<String> {
    match package_id {
        Some(value) => {
            let slug = slugify_package_id(value.trim());
            if slug.is_empty() {
                return Err(AppError::Validation(format!(
                    "package id `{value}` does not contain any usable ASCII letters or digits"
                )));
            }
            Ok(slug)
        }
        None if addon_directories.len() == 1 => {
            let slug = slugify_package_id(&addon_directories[0]);
            if slug.is_empty() {
                return Err(AppError::Validation(format!(
                    "addon directory `{}` could not be converted into a tracked package id",
                    addon_directories[0]
                )));
            }
            Ok(slug)
        }
        None => Err(AppError::Validation(
            "package id is required when adopting multiple addon directories into one tracked package"
                .to_string(),
        )),
    }
}

fn ensure_package_id_is_available(registry: &AddonRegistry, package_id: &str) -> AppResult<()> {
    if registry
        .packages
        .iter()
        .any(|package| package.package_id.eq_ignore_ascii_case(package_id))
    {
        return Err(AppError::Validation(format!(
            "tracked addon package `{package_id}` already exists"
        )));
    }

    Ok(())
}

fn resolve_snapshot_archive_path(
    state_paths: &super::AddonStatePaths,
    archive_output_path: Option<PathBuf>,
    package_id: &str,
) -> AppResult<PathBuf> {
    let path = archive_output_path
        .unwrap_or_else(|| state_paths.adopted_dir.join(format!("{package_id}.zip")));

    let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
        return Err(AppError::Validation(format!(
            "adoption archive path must point to a .zip file: {}",
            path.display()
        )));
    };
    if file_name.trim().is_empty() || !file_name.to_ascii_lowercase().ends_with(".zip") {
        return Err(AppError::Validation(format!(
            "adoption archive path must point to a .zip file: {}",
            path.display()
        )));
    }

    Ok(path)
}

fn ensure_archive_path_is_available(path: &Path) -> AppResult<()> {
    if path.exists() {
        return Err(AppError::Validation(format!(
            "adoption archive already exists: {}",
            path.display()
        )));
    }

    Ok(())
}

fn ensure_archive_path_is_outside_selected_addons(
    installation: &DetectedFlavorInstallation,
    archive_path: &Path,
    addon_directories: &[String],
) -> AppResult<()> {
    for addon_directory in addon_directories {
        let addon_path = installation.addon_dir.join(addon_directory);
        if archive_path.starts_with(&addon_path) {
            return Err(AppError::Validation(format!(
                "adoption archive path `{}` must not be placed inside adopted addon directory `{}`",
                archive_path.display(),
                addon_directory
            )));
        }
    }

    Ok(())
}

fn inspect_selected_addons(
    installation: &DetectedFlavorInstallation,
    addon_directories: &[String],
) -> AppResult<Vec<TrackedAddon>> {
    let mut addons = addon_directories
        .iter()
        .map(|addon_name| {
            inspect_addon_directory(&installation.addon_dir.join(addon_name), addon_name)
        })
        .collect::<AppResult<Vec<_>>>()?;
    addons.sort_by(|left, right| left.directory_name.cmp(&right.directory_name));
    Ok(addons)
}

fn planned_source_ref(path: &Path) -> AddonSourceRef {
    AddonSourceRef::LocalArchive {
        path: path.to_path_buf(),
    }
}

fn write_snapshot_archive(
    installation: &DetectedFlavorInstallation,
    addon_directories: &[String],
    archive_path: &Path,
) -> AppResult<()> {
    let archive_parent = archive_path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(archive_parent)?;

    let mut temporary = NamedTempFile::new_in(archive_parent)?;
    {
        let file = temporary.as_file_mut();
        let mut zip = ZipWriter::new(file);
        let mut archive_outputs = PortableArchivePathSet::new();

        for addon_directory in addon_directories {
            let addon_root = installation.addon_dir.join(addon_directory);
            append_addon_directory_to_zip(
                &mut zip,
                &addon_root,
                addon_directory,
                &mut archive_outputs,
            )?;
        }

        zip.finish()?;
    }

    temporary
        .persist(archive_path)
        .map_err(|error| error.error)?;
    Ok(())
}

fn append_addon_directory_to_zip(
    zip: &mut ZipWriter<&mut File>,
    addon_root: &Path,
    addon_directory: &str,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<()> {
    for entry in WalkDir::new(addon_root).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let path = entry.path();
        let file_type = entry.file_type();
        reject_unsupported_symlink_metadata(
            "addon adoption source",
            &path.display().to_string(),
            file_type.is_symlink(),
        )?;

        let relative = path
            .strip_prefix(addon_root)
            .map_err(|error| AppError::Validation(error.to_string()))?;
        if relative.as_os_str().is_empty() {
            continue;
        }

        let archive_path = Path::new(addon_directory).join(relative);
        if file_type.is_dir() {
            let archive_name = to_zip_path(&archive_path);
            archive_outputs
                .register(&archive_name, true)
                .map_err(|issue| portable_archive_path_issue_error("addon adoption", issue))?;
            add_directory_to_zip(zip, &archive_name, zip_dir_options())?;
            continue;
        }

        write_file_to_zip(zip, path, &archive_path, archive_outputs)?;
    }

    Ok(())
}

fn write_file_to_zip(
    zip: &mut ZipWriter<&mut File>,
    source_path: &Path,
    archive_path: &Path,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<()> {
    let archive_name = to_zip_path(archive_path);
    archive_outputs
        .register(&archive_name, false)
        .map_err(|issue| portable_archive_path_issue_error("addon adoption", issue))?;
    stream_file_to_zip(zip, source_path, &archive_name, zip_file_options())
}

fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644)
}

fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755)
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}
