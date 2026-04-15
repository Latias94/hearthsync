pub mod index;
pub mod lock;
mod provider;
mod registry;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tempfile::{TempDir, tempdir};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;
use zip::ZipArchive;

pub use self::provider::AddonSourceRef;
use self::provider::{
    AddonProviderContext, AddonSearchResult, materialize_source_input, materialize_source_ref,
    search_addons as search_provider_addons,
};
use self::registry::registry_path;
use crate::core::backup::{BackupGroup, BackupRequest, create_backup, restore_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

pub(crate) use self::registry::{load_registry, save_registry};

#[derive(Debug, Clone, Serialize)]
pub struct AddonInventory {
    pub target_addon_root: PathBuf,
    pub registry_path: PathBuf,
    pub tracked_packages: Vec<TrackedAddonPackage>,
    pub untracked_addons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub source: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub metadata: Option<AddonPackageMetadata>,
}

#[derive(Debug)]
pub(crate) struct InstallPreparedAddonRequest {
    pub(crate) installation: DetectedFlavorInstallation,
    pub(crate) prepared: PreparedAddonPackage,
    pub(crate) dry_run: bool,
    pub(crate) backup_output_path: Option<PathBuf>,
    pub(crate) replace_existing: bool,
    pub(crate) metadata: Option<AddonPackageMetadata>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledAddonPackageResult {
    pub dry_run: bool,
    pub source: AddonSourceRef,
    pub package_id: String,
    pub addons: Vec<TrackedAddon>,
    pub files_to_write: usize,
    pub written_files: usize,
    pub replaced_addons: Vec<String>,
    pub registry_path: PathBuf,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoveAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchAddonRequest {
    pub installation: DetectedFlavorInstallation,
    pub query: String,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchCatalog {
    pub query: String,
    pub results: Vec<AddonSearchResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdatedAddonPackageResult {
    pub dry_run: bool,
    pub registry_path: PathBuf,
    pub files_to_write: usize,
    pub written_files: usize,
    pub updated_packages: Vec<TrackedAddonPackage>,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemovedAddonPackageResult {
    pub dry_run: bool,
    pub registry_path: PathBuf,
    pub removed_packages: Vec<TrackedAddonPackage>,
    pub removed_addons: Vec<String>,
    pub registry_cleaned: bool,
    pub backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackedAddonPackage {
    pub package_id: String,
    pub source: AddonSourceRef,
    pub installed_at: String,
    pub updated_at: String,
    pub addons: Vec<TrackedAddon>,
    #[serde(default)]
    pub metadata: Option<AddonPackageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackedAddon {
    pub directory_name: String,
    pub toc_file: Option<String>,
    pub title: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AddonPackageMetadata {
    #[serde(default)]
    pub index_name: Option<String>,
    #[serde(default)]
    pub index_package_id: Option<String>,
    #[serde(default)]
    pub package_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    #[serde(default)]
    pub supported_flavors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AddonRegistry {
    schema_version: u32,
    packages: Vec<TrackedAddonPackage>,
}

impl Default for AddonRegistry {
    fn default() -> Self {
        Self {
            schema_version: 1,
            packages: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct PreparedAddonPackage {
    pub(crate) source: AddonSourceRef,
    pub(crate) package_id: String,
    pub(crate) addons: Vec<PreparedAddonDirectory>,
    pub(crate) metadata: Option<AddonPackageMetadata>,
    _stage_dir: TempDir,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedAddonDirectory {
    pub(crate) addon: TrackedAddon,
    stage_path: PathBuf,
    pub(crate) file_count: usize,
}

pub fn list_addons(installation: &DetectedFlavorInstallation) -> AppResult<AddonInventory> {
    let registry_path = registry_path(installation);
    let registry = load_registry(installation)?;
    let tracked_addons = registry
        .packages
        .iter()
        .flat_map(|package| {
            package
                .addons
                .iter()
                .map(|addon| addon.directory_name.clone())
        })
        .collect::<BTreeSet<_>>();
    let mut untracked_addons = discover_addon_directories(&installation.addon_dir)?
        .into_iter()
        .filter(|name| !tracked_addons.contains(name))
        .collect::<Vec<_>>();
    untracked_addons.sort();

    Ok(AddonInventory {
        target_addon_root: installation.addon_dir.clone(),
        registry_path,
        tracked_packages: registry.packages,
        untracked_addons,
    })
}

pub fn search_addons(request: SearchAddonRequest) -> AppResult<AddonSearchCatalog> {
    let results =
        search_provider_addons(&request.query, request.installation.flavor, request.limit)?;
    Ok(AddonSearchCatalog {
        query: request.query,
        results,
    })
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

fn prepare_package_from_source_input_with_flavor(
    source: &str,
    target_flavor: Option<crate::core::install::WowFlavor>,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    let materialized = materialize_source_input(
        source,
        stage_dir.path(),
        AddonProviderContext { target_flavor },
    )?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_source_ref_with_flavor(
    source: &AddonSourceRef,
    target_flavor: Option<crate::core::install::WowFlavor>,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    let materialized = materialize_source_ref(
        source,
        stage_dir.path(),
        AddonProviderContext { target_flavor },
    )?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_archive_with_source(
    source: AddonSourceRef,
    archive_path: &Path,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    prepare_package_from_archive(source, archive_path, stage_dir)
}

fn prepare_package_from_archive(
    source: AddonSourceRef,
    archive_path: &Path,
    stage_dir: TempDir,
) -> AppResult<PreparedAddonPackage> {
    let addons = extract_archive_addons(archive_path, stage_dir.path())?;
    if addons.is_empty() {
        return Err(AppError::Validation(
            "archive does not contain any detectable addon directories".to_string(),
        ));
    }

    let addon_names = addons
        .iter()
        .map(|addon| addon.addon.directory_name.as_str())
        .collect::<Vec<_>>();
    let package_id = derive_package_id(&source, &addon_names);

    Ok(PreparedAddonPackage {
        source,
        package_id,
        addons,
        metadata: None,
        _stage_dir: stage_dir,
    })
}

fn extract_archive_addons(
    archive_path: &Path,
    stage_root: &Path,
) -> AppResult<Vec<PreparedAddonDirectory>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;
    let addon_roots = discover_archive_addon_roots(&mut archive)?;
    if addon_roots.is_empty() {
        return Ok(Vec::new());
    }

    let mut file_counts = vec![0usize; addon_roots.len()];

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?;
        if segments.is_empty() {
            continue;
        }

        let Some(root_index) = match_addon_root(&segments, &addon_roots) else {
            continue;
        };
        let root = &addon_roots[root_index];
        let addon_name = root
            .last()
            .map(String::as_str)
            .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))?;
        let relative = &segments[root.len()..];
        let destination =
            join_segments(stage_root, &[addon_name]).join(join_segments(Path::new(""), relative));
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = File::create(destination)?;
        std::io::copy(&mut entry, &mut output)?;
        file_counts[root_index] += 1;
    }

    let mut prepared = Vec::new();
    for (index, root) in addon_roots.iter().enumerate() {
        let addon_name = root
            .last()
            .map(String::as_str)
            .ok_or_else(|| AppError::Validation("invalid addon root".to_string()))?;
        let stage_path = stage_root.join(addon_name);
        let addon = inspect_staged_addon(&stage_path, addon_name)?;
        prepared.push(PreparedAddonDirectory {
            addon,
            stage_path,
            file_count: file_counts[index],
        });
    }

    prepared.sort_by(|left, right| left.addon.directory_name.cmp(&right.addon.directory_name));
    Ok(prepared)
}

fn discover_archive_addon_roots(archive: &mut ZipArchive<File>) -> AppResult<Vec<Vec<String>>> {
    let mut roots = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }

        let entry_name = entry.name().to_string();
        let segments = safe_zip_segments(&entry_name)?;
        if segments.len() < 2 {
            continue;
        }

        let file_name = segments
            .last()
            .copied()
            .ok_or_else(|| AppError::Validation("invalid archive entry".to_string()))?;
        if !file_name.ends_with(".toc") {
            continue;
        }

        let Some(file_stem) = Path::new(file_name)
            .file_stem()
            .and_then(|stem| stem.to_str())
        else {
            continue;
        };
        let parent_name = segments[segments.len() - 2];
        if file_stem != parent_name {
            continue;
        }

        let root = segments[..segments.len() - 1]
            .iter()
            .map(|segment| (*segment).to_string())
            .collect::<Vec<_>>();
        if !roots.contains(&root) {
            roots.push(root);
        }
    }

    roots.sort_by(|left, right| right.len().cmp(&left.len()).then_with(|| left.cmp(right)));
    Ok(roots)
}

fn match_addon_root(segments: &[&str], roots: &[Vec<String>]) -> Option<usize> {
    roots.iter().position(|root| {
        root.len() <= segments.len()
            && root
                .iter()
                .zip(segments.iter())
                .all(|(left, right)| left.as_str() == *right)
    })
}

fn inspect_staged_addon(stage_path: &Path, addon_name: &str) -> AppResult<TrackedAddon> {
    let toc_path = find_primary_toc(stage_path, addon_name)?;
    let (toc_file, title, version) = if let Some(path) = toc_path {
        let toc_file = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string());
        let content = fs::read_to_string(&path).unwrap_or_default();
        let title = extract_toc_field(&content, "Title");
        let version = extract_toc_field(&content, "Version");
        (toc_file, title, version)
    } else {
        (None, None, None)
    };

    Ok(TrackedAddon {
        directory_name: addon_name.to_string(),
        toc_file,
        title,
        version,
    })
}

fn find_primary_toc(stage_path: &Path, addon_name: &str) -> AppResult<Option<PathBuf>> {
    if !stage_path.exists() {
        return Ok(None);
    }

    let preferred = stage_path.join(format!("{addon_name}.toc"));
    if preferred.exists() {
        return Ok(Some(preferred));
    }

    for entry in fs::read_dir(stage_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("toc"))
        {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

fn extract_toc_field(content: &str, field: &str) -> Option<String> {
    let needle = format!("## {field}:");
    content.lines().find_map(|line| {
        line.trim()
            .strip_prefix(&needle)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
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
            fs::create_dir_all(destination)?;
            continue;
        }

        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(path, &target)?;
        written_files += 1;
    }

    Ok(written_files)
}

fn remove_path(path: &Path) -> AppResult<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn discover_addon_directories(addon_dir: &Path) -> AppResult<Vec<String>> {
    if !addon_dir.exists() {
        return Ok(Vec::new());
    }

    let mut addons = Vec::new();
    for entry in fs::read_dir(addon_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".hearthsync" {
            continue;
        }

        if find_primary_toc(&path, &name)?.is_some() {
            addons.push(name);
        }
    }

    addons.sort();
    Ok(addons)
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

fn derive_package_id(source: &AddonSourceRef, addon_names: &[&str]) -> String {
    let base = match source {
        AddonSourceRef::LocalArchive { path } => path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
        AddonSourceRef::HttpArchive { url } => Path::new(url)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string()),
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => Some(match file_id {
            Some(file_id) => format!("curseforge-{mod_id}-{file_id}"),
            None => format!("curseforge-{mod_id}"),
        }),
        AddonSourceRef::GitHubRelease {
            owner,
            repo,
            tag,
            asset_name,
        } => asset_name
            .as_deref()
            .and_then(|value| Path::new(value).file_stem().and_then(|stem| stem.to_str()))
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
            .or_else(|| {
                tag.as_ref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| format!("{repo}-{value}"))
            })
            .or_else(|| Some(format!("{owner}-{repo}"))),
    }
    .or_else(|| addon_names.first().map(|name| (*name).to_string()))
    .unwrap_or_else(|| "addon-package".to_string());

    let mut slug = String::new();
    for character in base.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_string()
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn safe_zip_segments(entry_name: &str) -> AppResult<Vec<&str>> {
    let mut segments = Vec::new();
    for segment in entry_name.split('/') {
        if segment.is_empty() {
            continue;
        }

        if segment == "." || segment == ".." || segment.contains('\\') {
            return Err(AppError::Validation(format!(
                "unsafe archive path: `{entry_name}`"
            )));
        }

        segments.push(segment);
    }

    Ok(segments)
}

fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}
