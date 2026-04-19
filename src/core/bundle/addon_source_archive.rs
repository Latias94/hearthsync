use std::path::{Path, PathBuf};

use tempfile::tempdir;
use zip::ZipWriter;

use super::zip_write::add_path_to_zip;
use super::*;

pub(super) fn resolve_addon_index_paths(
    addon_indexes: &[String],
    manifest_base_dir: Option<&Path>,
) -> AppResult<Vec<(String, PathBuf)>> {
    let mut resolved = Vec::new();
    let mut file_names = Vec::new();

    for addon_index in addon_indexes {
        let reference = Path::new(addon_index);
        let source_path = if reference.is_absolute() {
            reference.to_path_buf()
        } else if let Some(base_dir) = manifest_base_dir {
            base_dir.join(reference)
        } else {
            return Err(AppError::Validation(format!(
                "relative addon index path requires `manifest_base_dir`: {addon_index}"
            )));
        };

        if !source_path.is_file() {
            return Err(AppError::NotFound(format!(
                "addon index file does not exist: {}",
                source_path.display()
            )));
        }

        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "addon index file has no usable file name: {}",
                    source_path.display()
                ))
            })?
            .to_string();
        validate_plain_name("addon index file", &file_name)?;
        if file_names.iter().any(|item| item == &file_name) {
            return Err(AppError::Validation(format!(
                "duplicate addon index file name in bundle metadata: {file_name}"
            )));
        }
        file_names.push(file_name.clone());
        resolved.push((file_name, source_path));
    }

    Ok(resolved)
}

pub(super) fn read_generated_addon_lock(path: &Path) -> AppResult<AddonLock> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

pub(super) fn add_bundle_addon_sources_to_zip(
    zip: &mut ZipWriter<std::fs::File>,
    installation: &DetectedFlavorInstallation,
    packages: &[AddonLockPackage],
) -> AppResult<BundleAddonSourceIndex> {
    let source_stage = tempdir()?;
    let mut entries = Vec::new();
    let mut used_file_names = Vec::new();
    let mut packages = packages.iter().collect::<Vec<_>>();
    packages.sort_by(|left, right| {
        addon_lock_package_comparison_key(left).cmp(&addon_lock_package_comparison_key(right))
    });

    for (index, package) in packages.into_iter().enumerate() {
        let comparison_key = addon_lock_package_comparison_key(package);
        let file_name = unique_bundle_source_archive_name(
            &comparison_key,
            &package.package_id,
            index,
            &mut used_file_names,
        );
        let source_archive_path = source_stage.path().join(&file_name);
        write_addon_package_source_archive(&source_archive_path, installation, package)?;
        let relative_source_path = format!("sources/{file_name}");
        let bundle_entry_path = Path::new(ADDON_SOURCE_ENTRY_ROOT).join(&file_name);
        add_path_to_zip(zip, &source_archive_path, &bundle_entry_path)?;

        entries.push(BundleAddonSourceEntry {
            comparison_key,
            package_id: package.package_id.clone(),
            path: relative_source_path,
            content_sha256: package.content_sha256.clone(),
            addon_directories: package.addon_directories.clone(),
        });
    }

    Ok(BundleAddonSourceIndex {
        schema_version: 1,
        sources: entries,
    })
}

fn unique_bundle_source_archive_name(
    comparison_key: &str,
    package_id: &str,
    index: usize,
    used_file_names: &mut Vec<String>,
) -> String {
    let mut base = safe_file_part(comparison_key);
    if base.is_empty() {
        base = safe_file_part(package_id);
    }
    if base.is_empty() {
        base = format!("package-{index}");
    }

    let mut candidate = format!("{base}.zip");
    let mut suffix = 2usize;
    while used_file_names.iter().any(|item| item == &candidate) {
        candidate = format!("{base}-{suffix}.zip");
        suffix += 1;
    }
    used_file_names.push(candidate.clone());
    candidate
}

fn write_addon_package_source_archive(
    archive_path: &Path,
    installation: &DetectedFlavorInstallation,
    package: &AddonLockPackage,
) -> AppResult<()> {
    let file = std::fs::File::create(archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archived_files = 0usize;

    for addon_directory in &package.addon_directories {
        validate_plain_name("addon", addon_directory)?;
        let source = installation.addon_dir.join(addon_directory);
        if !source.is_dir() {
            return Err(AppError::NotFound(format!(
                "tracked addon directory does not exist: {}",
                source.display()
            )));
        }
        archived_files += add_path_to_zip(&mut zip, &source, Path::new(addon_directory))?;
    }

    zip.finish()?;
    if archived_files == 0 {
        return Err(AppError::Validation(format!(
            "tracked package `{}` does not contain any addon files",
            package.package_id
        )));
    }

    Ok(())
}
