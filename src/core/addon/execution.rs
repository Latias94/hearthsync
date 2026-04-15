use std::collections::BTreeSet;
use std::path::Path;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use super::package_prep::prepare_package_from_source_input_with_flavor;
use super::*;
use crate::core::backup::{BackupGroup, BackupRequest, create_backup, restore_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug)]
pub(crate) struct InstallPreparedAddonRequest {
    pub(crate) installation: DetectedFlavorInstallation,
    pub(crate) prepared: PreparedAddonPackage,
    pub(crate) dry_run: bool,
    pub(crate) backup_output_path: Option<PathBuf>,
    pub(crate) replace_existing: bool,
    pub(crate) metadata: Option<AddonPackageMetadata>,
}

pub fn install_addon(request: InstallAddonRequest) -> AppResult<InstalledAddonPackageResult> {
    let prepared = prepare_package_from_source_input_with_flavor(
        &request.source,
        Some(request.installation.flavor),
    )?;
    install_prepared_addon(InstallPreparedAddonRequest {
        installation: request.installation,
        prepared,
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        metadata: request.metadata,
    })
}

pub(crate) fn install_prepared_addon(
    request: InstallPreparedAddonRequest,
) -> AppResult<InstalledAddonPackageResult> {
    let registry_path = registry_path(&request.installation);
    let mut prepared = request.prepared;
    prepared.metadata = request.metadata;
    let files_to_write = prepared
        .addons
        .iter()
        .map(|addon| addon.file_count)
        .sum::<usize>();
    let replaced_addons = prepared
        .addons
        .iter()
        .filter(|addon| {
            request
                .installation
                .addon_dir
                .join(&addon.addon.directory_name)
                .exists()
        })
        .map(|addon| addon.addon.directory_name.clone())
        .collect::<Vec<_>>();

    if !request.replace_existing && !replaced_addons.is_empty() {
        return Err(AppError::Validation(format!(
            "addon directories already exist: {}. Use `--replace-existing` or `addon update`.",
            replaced_addons.join(", ")
        )));
    }

    if request.dry_run {
        return Ok(InstalledAddonPackageResult {
            dry_run: true,
            source: prepared.source,
            package_id: prepared.package_id,
            addons: prepared
                .addons
                .into_iter()
                .map(|addon| addon.addon)
                .collect(),
            files_to_write,
            written_files: 0,
            replaced_addons,
            registry_path,
            backup_path: None,
        });
    }

    let backup_path = Some(
        create_backup(BackupRequest {
            installation: request.installation.clone(),
            output_path: request.backup_output_path,
            groups: vec![BackupGroup::Addons],
            label: Some("addon-install".to_string()),
        })?
        .archive_path,
    );

    match install_prepared_package(&request.installation, prepared, request.replace_existing) {
        Ok((package, written_files)) => Ok(InstalledAddonPackageResult {
            dry_run: false,
            source: package.source.clone(),
            package_id: package.package_id.clone(),
            addons: package.addons.clone(),
            files_to_write,
            written_files,
            replaced_addons,
            registry_path,
            backup_path,
        }),
        Err(error) => {
            rollback_or_report_addon_error(error, backup_path.as_deref(), &request.installation)
        }
    }
}

pub fn update_addons(request: UpdateAddonRequest) -> AppResult<UpdatedAddonPackageResult> {
    let registry_path = registry_path(&request.installation);
    let registry = load_registry(&request.installation)?;
    if registry.packages.is_empty() {
        return Err(AppError::Validation(
            "no tracked addon packages found. Use `addon install` first.".to_string(),
        ));
    }

    let selected_packages = select_packages_for_update(&registry, request.name.as_deref())?;
    let mut prepared_packages = Vec::new();
    for package in &selected_packages {
        prepared_packages.push(prepare_package_from_source_ref_with_flavor(
            &package.source,
            Some(request.installation.flavor),
        )?);
    }

    let files_to_write = prepared_packages
        .iter()
        .map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.file_count)
                .sum::<usize>()
        })
        .sum::<usize>();

    if request.dry_run {
        return Ok(UpdatedAddonPackageResult {
            dry_run: true,
            registry_path,
            files_to_write,
            written_files: 0,
            updated_packages: prepared_packages
                .into_iter()
                .zip(selected_packages.iter())
                .map(|(package, selected)| {
                    let metadata = package
                        .metadata
                        .clone()
                        .or_else(|| selected.metadata.clone());
                    TrackedAddonPackage {
                        package_id: package.package_id,
                        source: package.source,
                        installed_at: selected.installed_at.clone(),
                        updated_at: String::new(),
                        addons: package
                            .addons
                            .into_iter()
                            .map(|addon| addon.addon)
                            .collect(),
                        metadata,
                    }
                })
                .collect(),
            backup_path: None,
        });
    }

    let backup_path = Some(
        create_backup(BackupRequest {
            installation: request.installation.clone(),
            output_path: request.backup_output_path,
            groups: vec![BackupGroup::Addons],
            label: Some("addon-update".to_string()),
        })?
        .archive_path,
    );

    match update_prepared_packages(
        &request.installation,
        registry,
        selected_packages,
        prepared_packages,
    ) {
        Ok((updated_packages, written_files)) => Ok(UpdatedAddonPackageResult {
            dry_run: false,
            registry_path,
            files_to_write,
            written_files,
            updated_packages,
            backup_path,
        }),
        Err(error) => {
            rollback_or_report_addon_error(error, backup_path.as_deref(), &request.installation)
        }
    }
}

pub fn remove_addons(request: RemoveAddonRequest) -> AppResult<RemovedAddonPackageResult> {
    let registry_path = registry_path(&request.installation);
    let registry = load_registry(&request.installation)?;
    if registry.packages.is_empty() {
        return Err(AppError::Validation(
            "no tracked addon packages found. Use `addon install` first.".to_string(),
        ));
    }

    let removed_packages = select_packages_for_update(&registry, Some(&request.name))?;
    let removed_addons = removed_packages
        .iter()
        .flat_map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.directory_name.clone())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    if request.dry_run {
        return Ok(RemovedAddonPackageResult {
            dry_run: true,
            registry_path,
            removed_packages,
            removed_addons,
            registry_cleaned: false,
            backup_path: None,
        });
    }

    let backup_path = Some(
        create_backup(BackupRequest {
            installation: request.installation.clone(),
            output_path: request.backup_output_path,
            groups: vec![BackupGroup::Addons],
            label: Some("addon-remove".to_string()),
        })?
        .archive_path,
    );

    match remove_selected_packages(&request.installation, removed_packages.clone()) {
        Ok(registry_cleaned) => Ok(RemovedAddonPackageResult {
            dry_run: false,
            registry_path,
            removed_packages,
            removed_addons,
            registry_cleaned,
            backup_path,
        }),
        Err(error) => {
            rollback_or_report_addon_error(error, backup_path.as_deref(), &request.installation)
        }
    }
}

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

fn select_packages_for_update(
    registry: &AddonRegistry,
    name: Option<&str>,
) -> AppResult<Vec<TrackedAddonPackage>> {
    match name {
        None => Ok(registry.packages.clone()),
        Some(name) => {
            let mut matches = registry
                .packages
                .iter()
                .filter(|package| {
                    package.package_id.eq_ignore_ascii_case(name)
                        || package
                            .addons
                            .iter()
                            .any(|addon| addon.directory_name.eq_ignore_ascii_case(name))
                })
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

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}
