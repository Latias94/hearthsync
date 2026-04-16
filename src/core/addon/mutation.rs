use std::collections::BTreeSet;
use std::path::Path;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use super::*;
use crate::core::backup::restore_backup;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

pub(crate) fn install_prepared_package(
    installation: &DetectedFlavorInstallation,
    prepared: PreparedAddonPackage,
    replace_existing: bool,
) -> AppResult<(TrackedAddonPackage, usize)> {
    let addon_names = prepared
        .addons
        .iter()
        .map(|addon| addon.addon.directory_name.clone())
        .collect::<BTreeSet<_>>();
    let mut written_files = 0usize;
    let mut registry = load_registry(installation)?;

    registry.packages.retain(|package| {
        !package
            .addons
            .iter()
            .any(|addon| addon_names.contains(&addon.directory_name))
    });

    for addon in &prepared.addons {
        let destination = installation.addon_dir.join(&addon.addon.directory_name);
        if destination.exists() {
            if !replace_existing {
                return Err(AppError::Validation(format!(
                    "addon directory already exists: {}",
                    destination.display()
                )));
            }
            remove_path(&destination)?;
        }
        written_files += copy_directory(&addon.stage_path, &destination)?;
    }

    let timestamp = now_rfc3339()?;
    let package = TrackedAddonPackage {
        package_id: prepared.package_id,
        source: prepared.source,
        installed_at: timestamp.clone(),
        updated_at: timestamp,
        addons: prepared
            .addons
            .into_iter()
            .map(|addon| addon.addon)
            .collect(),
        metadata: prepared.metadata,
    };
    registry.packages.push(package.clone());
    save_registry(installation, &registry)?;

    Ok((package, written_files))
}

pub(crate) fn update_prepared_packages(
    installation: &DetectedFlavorInstallation,
    mut registry: AddonRegistry,
    selected_packages: Vec<TrackedAddonPackage>,
    prepared_packages: Vec<PreparedAddonPackage>,
) -> AppResult<(Vec<TrackedAddonPackage>, usize)> {
    let mut updated_packages = Vec::new();
    let mut written_files = 0usize;

    for (existing_package, prepared_package) in selected_packages.into_iter().zip(prepared_packages)
    {
        for addon in &existing_package.addons {
            let path = installation.addon_dir.join(&addon.directory_name);
            if path.exists() {
                remove_path(&path)?;
            }
        }

        for addon in &prepared_package.addons {
            let destination = installation.addon_dir.join(&addon.addon.directory_name);
            if destination.exists() {
                remove_path(&destination)?;
            }
            written_files += copy_directory(&addon.stage_path, &destination)?;
        }

        registry
            .packages
            .retain(|candidate| candidate != &existing_package);
        let timestamp = now_rfc3339()?;
        let updated_package = TrackedAddonPackage {
            package_id: prepared_package.package_id,
            source: prepared_package.source,
            installed_at: existing_package.installed_at,
            updated_at: timestamp,
            addons: prepared_package
                .addons
                .into_iter()
                .map(|addon| addon.addon)
                .collect(),
            metadata: prepared_package.metadata.or(existing_package.metadata),
        };
        registry.packages.push(updated_package.clone());
        updated_packages.push(updated_package);
    }

    save_registry(installation, &registry)?;
    Ok((updated_packages, written_files))
}

pub(crate) fn remove_selected_packages(
    installation: &DetectedFlavorInstallation,
    selected_packages: Vec<TrackedAddonPackage>,
) -> AppResult<bool> {
    let mut registry = load_registry(installation)?;

    for package in &selected_packages {
        for addon in &package.addons {
            let path = installation.addon_dir.join(&addon.directory_name);
            if path.exists() {
                remove_path(&path)?;
            }
        }
    }

    registry.packages.retain(|candidate| {
        !selected_packages
            .iter()
            .any(|selected| selected == candidate)
    });
    save_registry(installation, &registry)?;

    Ok(registry.packages.is_empty())
}

pub(crate) fn rollback_or_report_addon_error<T>(
    error: AppError,
    backup_path: Option<&Path>,
    installation: &DetectedFlavorInstallation,
) -> AppResult<T> {
    let Some(backup_path) = backup_path else {
        return Err(error);
    };

    match restore_backup(backup_path, installation) {
        Ok(restored) => Err(AppError::Validation(format!(
            "addon apply failed and rollback restored `{}` ({} files): {error}",
            restored.archive_path.display(),
            restored.restored_files
        ))),
        Err(rollback_error) => Err(AppError::Validation(format!(
            "addon apply failed: {error}; rollback failed: {rollback_error}"
        ))),
    }
}

fn copy_directory(source: &Path, destination: &Path) -> AppResult<usize> {
    let mut written_files = 0usize;

    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|error| AppError::Validation(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source)
            .map_err(|error| AppError::Validation(error.to_string()))?;

        if relative.as_os_str().is_empty() {
            std::fs::create_dir_all(destination)?;
            continue;
        }

        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }

        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(path, &target)?;
        written_files += 1;
    }

    Ok(written_files)
}

fn remove_path(path: &Path) -> AppResult<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}
