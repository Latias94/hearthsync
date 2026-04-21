use std::collections::BTreeSet;
use std::path::Path;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use crate::core::backup::restore_backup;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::task::{
    CancellationToken, TaskKind, TaskPhase, TaskProgressCode, TaskProgressSink,
    emit_task_step_progress, ensure_task_not_cancelled,
};

use super::{
    AddonRegistry, PreparedAddonPackage, TrackedAddonPackage, load_registry, save_registry,
};

#[derive(Clone, Copy)]
enum MutationProgressMode {
    Install,
    Update,
    Remove,
}

#[derive(Clone, Copy)]
enum AddonMutationStep<'a> {
    RemoveAddonDirectory {
        addon_name: &'a str,
        current: usize,
        total: usize,
    },
    WriteAddonDirectory {
        addon_name: &'a str,
        current: usize,
        total: usize,
    },
}

trait AddonMutationObserver {
    fn before_step(&mut self, _step: AddonMutationStep<'_>) -> AppResult<()> {
        Ok(())
    }
}

struct TaskAddonMutationObserver<'a, TCancel, TProgress> {
    task: TaskKind,
    mode: MutationProgressMode,
    cancellation: &'a TCancel,
    progress: &'a mut TProgress,
}

impl<'a, TCancel, TProgress> TaskAddonMutationObserver<'a, TCancel, TProgress> {
    fn new(
        task: TaskKind,
        mode: MutationProgressMode,
        cancellation: &'a TCancel,
        progress: &'a mut TProgress,
    ) -> Self {
        Self {
            task,
            mode,
            cancellation,
            progress,
        }
    }
}

impl<TCancel, TProgress> AddonMutationObserver for TaskAddonMutationObserver<'_, TCancel, TProgress>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    fn before_step(&mut self, step: AddonMutationStep<'_>) -> AppResult<()> {
        ensure_task_not_cancelled(self.cancellation, self.task, TaskPhase::Executing)?;
        let (code, current, total) = addon_mutation_step_progress(self.mode, step);
        emit_task_step_progress(
            self.progress,
            self.task,
            TaskPhase::Executing,
            code,
            current,
            total,
            addon_mutation_step_message(self.mode, step),
        );
        Ok(())
    }
}

pub(crate) fn install_prepared_package_task<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
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
    install_prepared_package_with_observer(installation, prepared, replace_existing, &mut observer)
}

fn install_prepared_package_with_observer(
    installation: &DetectedFlavorInstallation,
    prepared: PreparedAddonPackage,
    replace_existing: bool,
    observer: &mut impl AddonMutationObserver,
) -> AppResult<(TrackedAddonPackage, usize)> {
    let addon_names = prepared
        .addons
        .iter()
        .map(|addon| addon.addon.directory_name.clone())
        .collect::<BTreeSet<_>>();
    let total_addons = prepared.addons.len();
    let mut written_files = 0usize;
    let mut registry = load_registry(installation)?;

    registry.packages.retain(|package| {
        !package
            .addons
            .iter()
            .any(|addon| addon_names.contains(&addon.directory_name))
    });

    for (index, addon) in prepared.addons.iter().enumerate() {
        observer.before_step(AddonMutationStep::WriteAddonDirectory {
            addon_name: &addon.addon.directory_name,
            current: index + 1,
            total: total_addons,
        })?;
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

pub(crate) fn update_prepared_packages_task<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
    registry: AddonRegistry,
    selected_packages: Vec<TrackedAddonPackage>,
    prepared_packages: Vec<PreparedAddonPackage>,
    task: TaskKind,
    cancellation: &TCancel,
    progress: &mut TProgress,
) -> AppResult<(Vec<TrackedAddonPackage>, usize)>
where
    TCancel: CancellationToken,
    TProgress: TaskProgressSink,
{
    let mut observer =
        TaskAddonMutationObserver::new(task, MutationProgressMode::Update, cancellation, progress);
    update_prepared_packages_with_observer(
        installation,
        registry,
        selected_packages,
        prepared_packages,
        &mut observer,
    )
}

fn update_prepared_packages_with_observer(
    installation: &DetectedFlavorInstallation,
    mut registry: AddonRegistry,
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
            let path = installation.addon_dir.join(&addon.directory_name);
            if path.exists() {
                remove_path(&path)?;
            }
        }

        for addon in &prepared_package.addons {
            written_addons += 1;
            observer.before_step(AddonMutationStep::WriteAddonDirectory {
                addon_name: &addon.addon.directory_name,
                current: written_addons,
                total: total_written_addons,
            })?;
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

pub(crate) fn remove_selected_packages_task<TCancel, TProgress>(
    installation: &DetectedFlavorInstallation,
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
    remove_selected_packages_with_observer(installation, selected_packages, &mut observer)
}

fn remove_selected_packages_with_observer(
    installation: &DetectedFlavorInstallation,
    selected_packages: Vec<TrackedAddonPackage>,
    observer: &mut impl AddonMutationObserver,
) -> AppResult<bool> {
    let mut registry = load_registry(installation)?;
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

fn addon_mutation_step_message(mode: MutationProgressMode, step: AddonMutationStep<'_>) -> String {
    match (mode, step) {
        (
            MutationProgressMode::Install,
            AddonMutationStep::WriteAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Installing addon directory {current}/{total} `{addon_name}`"),
        (
            MutationProgressMode::Update,
            AddonMutationStep::RemoveAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Removing existing addon directory {current}/{total} `{addon_name}`"),
        (
            MutationProgressMode::Update,
            AddonMutationStep::WriteAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Writing updated addon directory {current}/{total} `{addon_name}`"),
        (
            MutationProgressMode::Remove,
            AddonMutationStep::RemoveAddonDirectory {
                addon_name,
                current,
                total,
            },
        ) => format!("Removing addon directory {current}/{total} `{addon_name}`"),
        (MutationProgressMode::Install, AddonMutationStep::RemoveAddonDirectory { .. })
        | (MutationProgressMode::Remove, AddonMutationStep::WriteAddonDirectory { .. }) => {
            "Applying addon mutation".to_string()
        }
    }
}

fn addon_mutation_step_progress(
    mode: MutationProgressMode,
    step: AddonMutationStep<'_>,
) -> (TaskProgressCode, usize, usize) {
    match (mode, step) {
        (
            MutationProgressMode::Install,
            AddonMutationStep::WriteAddonDirectory { current, total, .. },
        )
        | (
            MutationProgressMode::Update,
            AddonMutationStep::WriteAddonDirectory { current, total, .. },
        ) => (TaskProgressCode::WriteAddonDirectory, current, total),
        (
            MutationProgressMode::Update,
            AddonMutationStep::RemoveAddonDirectory { current, total, .. },
        )
        | (
            MutationProgressMode::Remove,
            AddonMutationStep::RemoveAddonDirectory { current, total, .. },
        ) => (TaskProgressCode::RemoveAddonDirectory, current, total),
        (MutationProgressMode::Install, AddonMutationStep::RemoveAddonDirectory { .. })
        | (MutationProgressMode::Remove, AddonMutationStep::WriteAddonDirectory { .. }) => {
            (TaskProgressCode::Executing, 1, 1)
        }
    }
}
