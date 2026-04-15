use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::addon::{
    AddonPackageMetadata, AddonSourceRef, InstallAddonRequest, InstalledAddonPackageResult,
    PreparedAddonPackage, TrackedAddonPackage, UpdatedAddonPackageResult, install_addon,
    list_addons, load_registry, prepare_package_from_source_ref_with_flavor,
    rollback_or_report_addon_error, update_prepared_packages,
};
use crate::core::backup::{BackupGroup, BackupRequest, create_backup};
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonIndex {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub packages: Vec<AddonIndexPackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonIndexPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: AddonSourceRef,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub addon_directories: Vec<String>,
    #[serde(default)]
    pub supported_flavors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspection {
    pub index_path: PathBuf,
    pub index: AddonIndex,
    pub package_count: usize,
}

#[derive(Debug, Clone)]
pub struct AddonIndexInstallRequest {
    pub installation: DetectedFlavorInstallation,
    pub index_path: PathBuf,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInstallResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackage,
    pub install: InstalledAddonPackageResult,
}

#[derive(Debug, Clone)]
pub struct AddonIndexUpdateRequest {
    pub installation: DetectedFlavorInstallation,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexUpdateResult {
    pub index_path: PathBuf,
    pub selected_packages: Vec<AddonIndexPackage>,
    pub update: UpdatedAddonPackageResult,
}

pub fn inspect_addon_index(path: &Path) -> AppResult<AddonIndexInspection> {
    let index = load_addon_index(path)?;
    let package_count = index.packages.len();

    Ok(AddonIndexInspection {
        index_path: path.to_path_buf(),
        index,
        package_count,
    })
}

pub fn install_addon_from_index(
    request: AddonIndexInstallRequest,
) -> AppResult<AddonIndexInstallResult> {
    let index = load_addon_index(&request.index_path)?;
    let package = find_index_package(&index, &request.name)?.clone();
    ensure_package_supports_flavor(&package, request.installation.flavor.as_str())?;
    let install = install_addon(InstallAddonRequest {
        installation: request.installation,
        source: package.source.display_name(),
        dry_run: request.dry_run,
        backup_output_path: request.backup_output_path,
        replace_existing: request.replace_existing,
        metadata: Some(metadata_from_index_package(&index, &package)),
    })?;

    Ok(AddonIndexInstallResult {
        index_path: request.index_path,
        package,
        install,
    })
}

pub fn update_addons_from_index(
    request: AddonIndexUpdateRequest,
) -> AppResult<AddonIndexUpdateResult> {
    let index = load_addon_index(&request.index_path)?;
    let selected_packages = match &request.name {
        Some(name) => vec![find_index_package(&index, name)?.clone()],
        None => index.packages.clone(),
    };
    for package in &selected_packages {
        ensure_package_supports_flavor(package, request.installation.flavor.as_str())?;
    }
    let inventory = list_addons(&request.installation)?;
    if inventory.tracked_packages.is_empty() {
        return Err(AppError::Validation(
            "no tracked addon packages found. Use `addon index install` or `addon install` first."
                .to_string(),
        ));
    }

    let mut prepared_packages = Vec::new();
    let mut matched_packages = Vec::new();
    let mut used_package_ids = BTreeSet::new();
    for package in &selected_packages {
        let mut prepared = prepare_package_from_source_ref_with_flavor(
            &package.source,
            Some(request.installation.flavor),
        )?;
        prepared.metadata = Some(metadata_from_index_package(&index, package));
        let matched = match_index_package_to_tracked_package(
            package,
            &prepared,
            &inventory.tracked_packages,
            &used_package_ids,
        )?;
        used_package_ids.insert(matched.package_id.clone());
        prepared_packages.push(prepared);
        matched_packages.push(matched);
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

    let update = if request.dry_run {
        UpdatedAddonPackageResult {
            dry_run: true,
            registry_path: inventory.registry_path,
            files_to_write,
            written_files: 0,
            updated_packages: preview_updated_packages(&matched_packages, &prepared_packages),
            backup_path: None,
        }
    } else {
        let registry = load_registry(&request.installation)?;
        let registry_path = inventory.registry_path;
        let backup_path = Some(
            create_backup(BackupRequest {
                installation: request.installation.clone(),
                output_path: request.backup_output_path,
                groups: vec![BackupGroup::Addons],
                label: Some("addon-index-update".to_string()),
            })?
            .archive_path,
        );

        match update_prepared_packages(
            &request.installation,
            registry,
            matched_packages,
            prepared_packages,
        ) {
            Ok((updated_packages, written_files)) => UpdatedAddonPackageResult {
                dry_run: false,
                registry_path,
                files_to_write,
                written_files,
                updated_packages,
                backup_path,
            },
            Err(error) => {
                return rollback_or_report_addon_error(
                    error,
                    backup_path.as_deref(),
                    &request.installation,
                );
            }
        }
    };

    Ok(AddonIndexUpdateResult {
        index_path: request.index_path,
        selected_packages,
        update,
    })
}

fn load_addon_index(path: &Path) -> AppResult<AddonIndex> {
    let content = fs::read_to_string(path)?;
    let index = toml::from_str::<AddonIndex>(&content)?;
    validate_addon_index(&index)?;
    Ok(index)
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

    let mut ids = Vec::new();
    for package in &index.packages {
        validate_index_package(package)?;
        if ids.iter().any(|id| id == &package.id) {
            return Err(AppError::Validation(format!(
                "duplicate addon index package id: {}",
                package.id
            )));
        }
        ids.push(package.id.clone());
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

fn find_index_package<'a>(index: &'a AddonIndex, name: &str) -> AppResult<&'a AddonIndexPackage> {
    index
        .packages
        .iter()
        .find(|package| {
            package.id.eq_ignore_ascii_case(name) || package.name.eq_ignore_ascii_case(name)
        })
        .ok_or_else(|| AppError::NotFound(format!("addon index package `{name}` not found")))
}

fn ensure_package_supports_flavor(package: &AddonIndexPackage, flavor: &str) -> AppResult<()> {
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

fn preview_updated_packages(
    matched_packages: &[TrackedAddonPackage],
    prepared_packages: &[PreparedAddonPackage],
) -> Vec<TrackedAddonPackage> {
    matched_packages
        .iter()
        .zip(prepared_packages.iter())
        .map(|(matched, prepared)| TrackedAddonPackage {
            package_id: prepared.package_id.clone(),
            source: prepared.source.clone(),
            installed_at: matched.installed_at.clone(),
            updated_at: String::new(),
            addons: prepared
                .addons
                .iter()
                .map(|addon| addon.addon.clone())
                .collect(),
            metadata: prepared
                .metadata
                .clone()
                .or_else(|| matched.metadata.clone()),
        })
        .collect()
}

fn metadata_from_index_package(
    index: &AddonIndex,
    package: &AddonIndexPackage,
) -> AddonPackageMetadata {
    AddonPackageMetadata {
        index_name: Some(index.name.clone()),
        index_package_id: Some(package.id.clone()),
        package_name: Some(package.name.clone()),
        version: Some(package.version.clone()),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.sha256.clone(),
        supported_flavors: package.supported_flavors.clone(),
    }
}

fn match_index_package_to_tracked_package(
    package: &AddonIndexPackage,
    prepared: &PreparedAddonPackage,
    tracked_packages: &[TrackedAddonPackage],
    used_package_ids: &BTreeSet<String>,
) -> AppResult<TrackedAddonPackage> {
    let expected_addon_names = expected_addon_names(package, prepared);

    let exact_id_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && candidate.package_id.eq_ignore_ascii_case(&package.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    if exact_id_matches.len() == 1 {
        return Ok(exact_id_matches[0].clone());
    }
    if exact_id_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by id",
            package.id
        )));
    }

    let full_matches = tracked_packages
        .iter()
        .filter(|candidate| {
            !used_package_ids.contains(&candidate.package_id)
                && tracked_package_contains_all_addons(candidate, &expected_addon_names)
        })
        .cloned()
        .collect::<Vec<_>>();
    if full_matches.len() == 1 {
        return Ok(full_matches[0].clone());
    }
    if full_matches.len() > 1 {
        return Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages by addon directories: {}",
            package.id,
            full_matches
                .iter()
                .map(|candidate| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }

    let mut partial_matches = tracked_packages
        .iter()
        .filter(|candidate| !used_package_ids.contains(&candidate.package_id))
        .filter_map(|candidate| {
            let overlap = tracked_package_addon_overlap(candidate, &expected_addon_names);
            (overlap > 0).then_some((overlap, candidate.clone()))
        })
        .collect::<Vec<_>>();
    partial_matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.package_id.cmp(&right.1.package_id))
    });

    match partial_matches.as_slice() {
        [] => Err(AppError::Validation(format!(
            "addon index package `{}` is not installed or not tracked locally",
            package.id
        ))),
        [(_, candidate)] => Ok(candidate.clone()),
        [(best_overlap, best), (next_overlap, next), ..] if best_overlap > next_overlap => {
            let _ = next;
            Ok(best.clone())
        }
        _ => Err(AppError::Validation(format!(
            "addon index package `{}` matched multiple tracked packages with the same confidence: {}",
            package.id,
            partial_matches
                .iter()
                .map(|(_, candidate)| candidate.package_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

fn expected_addon_names(
    package: &AddonIndexPackage,
    prepared: &PreparedAddonPackage,
) -> BTreeSet<String> {
    let addon_names = if package.addon_directories.is_empty() {
        prepared
            .addons
            .iter()
            .map(|addon| addon.addon.directory_name.clone())
            .collect::<Vec<_>>()
    } else {
        package.addon_directories.clone()
    };

    addon_names
        .into_iter()
        .map(|name| name.trim().to_ascii_lowercase())
        .filter(|name| !name.is_empty())
        .collect()
}

fn tracked_package_contains_all_addons(
    candidate: &TrackedAddonPackage,
    expected_addon_names: &BTreeSet<String>,
) -> bool {
    !expected_addon_names.is_empty()
        && candidate
            .addons
            .iter()
            .map(|addon| addon.directory_name.trim().to_ascii_lowercase())
            .collect::<BTreeSet<_>>()
            .is_superset(expected_addon_names)
}

fn tracked_package_addon_overlap(
    candidate: &TrackedAddonPackage,
    expected_addon_names: &BTreeSet<String>,
) -> usize {
    candidate
        .addons
        .iter()
        .map(|addon| addon.directory_name.trim().to_ascii_lowercase())
        .filter(|name| expected_addon_names.contains(name))
        .count()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    use tempfile::tempdir;
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{
        AddonIndexInstallRequest, inspect_addon_index, install_addon_from_index,
        update_addons_from_index,
    };
    use crate::core::addon::index::AddonIndexUpdateRequest;
    use crate::core::addon::{AddonSourceRef, InstallAddonRequest, install_addon, list_addons};
    use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

    #[test]
    fn inspect_addon_index_reads_packages() {
        let temp = tempdir().expect("temp dir");
        let archive_path = temp.path().join("details.zip");
        let index_path = write_index(temp.path(), &archive_path);

        let inspection = inspect_addon_index(&index_path).expect("inspect index");

        assert_eq!(inspection.index.name, "Fixture Index");
        assert_eq!(inspection.package_count, 1);
        assert_eq!(inspection.index.packages[0].id, "details");
    }

    #[test]
    fn install_addon_from_index_installs_selected_package() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let archive_path = temp.path().join("details.zip");
        create_addon_archive(
            &archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );
        let index_path = write_index(temp.path(), &archive_path);

        let result = install_addon_from_index(AddonIndexInstallRequest {
            installation: installation.clone(),
            index_path,
            name: "details".to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
        })
        .expect("install from index");

        assert_eq!(result.package.id, "details");
        assert!(
            installation
                .addon_dir
                .join("Details")
                .join("Details.toc")
                .exists()
        );
    }

    #[test]
    fn update_addons_from_index_uses_index_source_and_skips_unselected_packages() {
        let temp = tempdir().expect("temp dir");
        let installation = create_fixture_installation(temp.path());
        let installed_archive_path = temp.path().join("details-installed.zip");
        let updated_archive_path = temp.path().join("details-updated.zip");
        let extra_archive_path = temp.path().join("omen.zip");
        create_addon_archive(
            &installed_archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 110000\n## Version: 1.0.0\n",
            )],
        );
        create_addon_archive(
            &updated_archive_path,
            &[(
                "Details/Details.toc",
                "## Interface: 120000\n## Version: 2.0.0\n",
            )],
        );
        create_addon_archive(
            &extra_archive_path,
            &[("Omen/Omen.toc", "## Interface: 110000\n## Version: 1.0.0\n")],
        );
        let index_path = write_index(temp.path(), &updated_archive_path);

        install_addon(InstallAddonRequest {
            installation: installation.clone(),
            source: installed_archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install details");
        install_addon(InstallAddonRequest {
            installation: installation.clone(),
            source: extra_archive_path.display().to_string(),
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
            replace_existing: false,
            metadata: None,
        })
        .expect("install omen");

        let result = update_addons_from_index(AddonIndexUpdateRequest {
            installation: installation.clone(),
            index_path,
            name: None,
            dry_run: false,
            backup_output_path: Some(temp.path().join("backups")),
        })
        .expect("update from index");

        assert_eq!(result.selected_packages.len(), 1);
        assert!(
            fs::read_to_string(installation.addon_dir.join("Details").join("Details.toc"))
                .expect("toc")
                .contains("2.0.0")
        );
        assert!(
            fs::read_to_string(installation.addon_dir.join("Omen").join("Omen.toc"))
                .expect("omen toc")
                .contains("1.0.0")
        );

        let inventory = list_addons(&installation).expect("inventory");
        let details_package = inventory
            .tracked_packages
            .iter()
            .find(|package| {
                package
                    .addons
                    .iter()
                    .any(|addon| addon.directory_name == "Details")
            })
            .expect("details package");
        assert_eq!(
            details_package.source,
            AddonSourceRef::LocalArchive {
                path: updated_archive_path,
            }
        );
    }

    fn create_fixture_installation(root: &Path) -> DetectedFlavorInstallation {
        let product_root = root.join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");
        let interface_dir = flavor_root.join("Interface");
        let addon_dir = interface_dir.join("AddOns");
        let wtf_dir = flavor_root.join("WTF");
        let fonts_dir = flavor_root.join("Fonts");

        fs::create_dir_all(&addon_dir).expect("addon dir");
        fs::create_dir_all(&wtf_dir).expect("wtf dir");
        fs::create_dir_all(&fonts_dir).expect("fonts dir");

        DetectedFlavorInstallation {
            platform: HostPlatform::Windows,
            product_root,
            flavor_root,
            flavor: WowFlavor::Retail,
            interface_dir,
            addon_dir,
            wtf_dir,
            fonts_dir,
        }
    }

    fn write_index(root: &Path, archive_path: &Path) -> std::path::PathBuf {
        let index_path = root.join("index.toml");
        fs::write(
            &index_path,
            format!(
                r#"
schema_version = 1
name = "Fixture Index"

[[packages]]
id = "details"
name = "Details"
version = "1.0.0"
source = {{ kind = "local_archive", path = "{}" }}
supported_flavors = ["retail"]
"#,
                archive_path.display().to_string().replace('\\', "\\\\")
            ),
        )
        .expect("index");
        index_path
    }

    fn create_addon_archive(path: &Path, entries: &[(&str, &str)]) {
        let file = fs::File::create(path).expect("archive file");
        let mut zip = ZipWriter::new(file);
        for (name, content) in entries {
            zip.start_file(
                name.replace('\\', "/"),
                SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .expect("start file");
            zip.write_all(content.as_bytes()).expect("write file");
        }
        zip.finish().expect("finish zip");
    }
}
