use std::collections::BTreeSet;
use std::path::Path;

use crate::core::backup::restore_backup;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{CancellationToken, TaskKind, TaskProgressSink};

mod fs;
mod progress;

use self::fs::{copy_directory, now_rfc3339, remove_path};
use self::progress::{
    AddonMutationObserver, AddonMutationStep, MutationProgressMode, TaskAddonMutationObserver,
};
use super::{
    AddonRegistry, AddonStatePaths, PreparedAddonPackage, TrackedAddonPackage,
    addon_directory_path_key, find_existing_addon_path, load_registry, save_registry,
};

pub(crate) struct UpdatePreparedPackagesWithDependenciesRequest {
    pub(crate) registry: AddonRegistry,
    pub(crate) selected_packages: Vec<TrackedAddonPackage>,
    pub(crate) prepared_packages: Vec<PreparedAddonPackage>,
    pub(crate) dependency_prepared_packages: Vec<PreparedAddonPackage>,
    pub(crate) task: TaskKind,
}

pub(crate) struct UpdatePreparedPackagesTaskRequest {
    pub(crate) registry: AddonRegistry,
    pub(crate) selected_packages: Vec<TrackedAddonPackage>,
    pub(crate) prepared_packages: Vec<PreparedAddonPackage>,
    pub(crate) task: TaskKind,
}

pub(crate) fn install_prepared_package_task<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    prepared: PreparedAddonPackage,
    replace_existing: bool,
    task: TaskKind,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<(TrackedAddonPackage, usize)>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let mut observer =
        TaskAddonMutationObserver::new(task, MutationProgressMode::Install, cancellation, progress);
    install_prepared_package_with_observer(
        installation,
        state_paths,
        prepared,
        replace_existing,
        &mut observer,
    )
}

fn install_prepared_package_with_observer(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    prepared: PreparedAddonPackage,
    replace_existing: bool,
    observer: &mut impl AddonMutationObserver,
) -> AppResult<(TrackedAddonPackage, usize)> {
    let mut registry = load_registry(installation, state_paths)?;
    let result = apply_install_prepared_package_with_observer(
        installation,
        &mut registry,
        prepared,
        replace_existing,
        observer,
    )?;
    save_registry(installation, state_paths, &registry)?;

    Ok(result)
}

fn apply_install_prepared_package_with_observer(
    installation: &DetectedFlavorInstallation,
    registry: &mut AddonRegistry,
    prepared: PreparedAddonPackage,
    replace_existing: bool,
    observer: &mut impl AddonMutationObserver,
) -> AppResult<(TrackedAddonPackage, usize)> {
    let addon_names = prepared
        .addons
        .iter()
        .map(|addon| addon_directory_path_key(&addon.addon.directory_name, installation.platform))
        .collect::<BTreeSet<_>>();
    let total_addons = prepared.addons.len();
    let mut written_files = 0usize;

    registry.packages.retain(|package| {
        !package.addons.iter().any(|addon| {
            addon_names.contains(&addon_directory_path_key(
                &addon.directory_name,
                installation.platform,
            ))
        })
    });

    for (index, addon) in prepared.addons.iter().enumerate() {
        observer.before_step(AddonMutationStep::WriteAddonDirectory {
            addon_name: &addon.addon.directory_name,
            current: index + 1,
            total: total_addons,
        })?;
        if let Some(existing) = find_existing_addon_path(
            &installation.addon_dir,
            &addon.addon.directory_name,
            installation.platform,
        )? {
            if !replace_existing {
                return Err(AppError::Validation(format!(
                    "addon directory already exists: {}",
                    existing.path.display()
                )));
            }
            remove_path(&existing.path)?;
        }
        let destination = installation.addon_dir.join(&addon.addon.directory_name);
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

    Ok((package, written_files))
}

pub(crate) fn update_prepared_packages_task<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    request: UpdatePreparedPackagesTaskRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<(Vec<TrackedAddonPackage>, usize)>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let UpdatePreparedPackagesTaskRequest {
        registry,
        selected_packages,
        prepared_packages,
        task,
    } = request;
    let mut observer =
        TaskAddonMutationObserver::new(task, MutationProgressMode::Update, cancellation, progress);
    update_prepared_packages_with_observer(
        installation,
        state_paths,
        registry,
        selected_packages,
        prepared_packages,
        &mut observer,
    )
}

pub(crate) fn update_prepared_packages_with_dependencies_task<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    request: UpdatePreparedPackagesWithDependenciesRequest,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<(Vec<TrackedAddonPackage>, Vec<TrackedAddonPackage>, usize)>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let UpdatePreparedPackagesWithDependenciesRequest {
        registry,
        selected_packages,
        prepared_packages,
        dependency_prepared_packages,
        task,
    } = request;
    let mut registry = registry;
    let (updated_packages, mut written_files) = {
        let mut observer = TaskAddonMutationObserver::new(
            task,
            MutationProgressMode::Update,
            cancellation,
            progress,
        );
        apply_update_prepared_packages_with_observer(
            installation,
            &mut registry,
            selected_packages,
            prepared_packages,
            &mut observer,
        )?
    };

    let mut installed_dependency_packages = Vec::new();
    {
        let mut observer = TaskAddonMutationObserver::new(
            task,
            MutationProgressMode::Install,
            cancellation,
            progress,
        );
        for prepared_dependency in dependency_prepared_packages {
            let (installed_dependency, installed_files) =
                apply_install_prepared_package_with_observer(
                    installation,
                    &mut registry,
                    prepared_dependency,
                    false,
                    &mut observer,
                )?;
            written_files += installed_files;
            installed_dependency_packages.push(installed_dependency);
        }
    }

    save_registry(installation, state_paths, &registry)?;

    Ok((
        updated_packages,
        installed_dependency_packages,
        written_files,
    ))
}

fn update_prepared_packages_with_observer(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    registry: AddonRegistry,
    selected_packages: Vec<TrackedAddonPackage>,
    prepared_packages: Vec<PreparedAddonPackage>,
    observer: &mut impl AddonMutationObserver,
) -> AppResult<(Vec<TrackedAddonPackage>, usize)> {
    let mut registry = registry;
    let result = apply_update_prepared_packages_with_observer(
        installation,
        &mut registry,
        selected_packages,
        prepared_packages,
        observer,
    )?;
    save_registry(installation, state_paths, &registry)?;

    Ok(result)
}

fn apply_update_prepared_packages_with_observer(
    installation: &DetectedFlavorInstallation,
    registry: &mut AddonRegistry,
    selected_packages: Vec<TrackedAddonPackage>,
    prepared_packages: Vec<PreparedAddonPackage>,
    observer: &mut impl AddonMutationObserver,
) -> AppResult<(Vec<TrackedAddonPackage>, usize)> {
    let mut updated_packages = Vec::new();
    let mut written_files = 0usize;
    let total_removed_addons = selected_packages
        .iter()
        .map(|package| package.addons.len())
        .sum::<usize>();
    let total_written_addons = prepared_packages
        .iter()
        .map(|package| package.addons.len())
        .sum::<usize>();
    let mut removed_addons = 0usize;
    let mut written_addons = 0usize;

    for (existing_package, prepared_package) in selected_packages.into_iter().zip(prepared_packages)
    {
        for addon in &existing_package.addons {
            removed_addons += 1;
            observer.before_step(AddonMutationStep::RemoveAddonDirectory {
                addon_name: &addon.directory_name,
                current: removed_addons,
                total: total_removed_addons,
            })?;
            if let Some(existing) = find_existing_addon_path(
                &installation.addon_dir,
                &addon.directory_name,
                installation.platform,
            )? {
                remove_path(&existing.path)?;
            }
        }

        for addon in &prepared_package.addons {
            written_addons += 1;
            observer.before_step(AddonMutationStep::WriteAddonDirectory {
                addon_name: &addon.addon.directory_name,
                current: written_addons,
                total: total_written_addons,
            })?;
            if let Some(existing) = find_existing_addon_path(
                &installation.addon_dir,
                &addon.addon.directory_name,
                installation.platform,
            )? {
                remove_path(&existing.path)?;
            }
            let destination = installation.addon_dir.join(&addon.addon.directory_name);
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

    Ok((updated_packages, written_files))
}

pub(crate) fn remove_selected_packages_task<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    selected_packages: Vec<TrackedAddonPackage>,
    task: TaskKind,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<bool>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let mut observer =
        TaskAddonMutationObserver::new(task, MutationProgressMode::Remove, cancellation, progress);
    remove_selected_packages_with_observer(
        installation,
        state_paths,
        selected_packages,
        &mut observer,
    )
}

fn remove_selected_packages_with_observer(
    installation: &DetectedFlavorInstallation,
    state_paths: &AddonStatePaths,
    selected_packages: Vec<TrackedAddonPackage>,
    observer: &mut impl AddonMutationObserver,
) -> AppResult<bool> {
    let mut registry = load_registry(installation, state_paths)?;
    let total_removed_addons = selected_packages
        .iter()
        .map(|package| package.addons.len())
        .sum::<usize>();
    let mut removed_addons = 0usize;

    for package in &selected_packages {
        for addon in &package.addons {
            removed_addons += 1;
            observer.before_step(AddonMutationStep::RemoveAddonDirectory {
                addon_name: &addon.directory_name,
                current: removed_addons,
                total: total_removed_addons,
            })?;
            if let Some(existing) = find_existing_addon_path(
                &installation.addon_dir,
                &addon.directory_name,
                installation.platform,
            )? {
                remove_path(&existing.path)?;
            }
        }
    }

    registry.packages.retain(|candidate| {
        !selected_packages
            .iter()
            .any(|selected| selected == candidate)
    });
    save_registry(installation, state_paths, &registry)?;

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
