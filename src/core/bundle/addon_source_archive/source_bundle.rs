use std::path::Path;

use tempfile::tempdir;
use zip::ZipWriter;

use super::super::constants::ADDON_SOURCE_ENTRY_ROOT;
use super::super::shared::addon_source_index::{BundleAddonSourceEntry, BundleAddonSourceIndex};
use super::super::shared::path::{safe_file_part, validate_plain_name};
use super::super::zip_write::add_path_to_zip;
use crate::core::addon::lock::{AddonLockPackage, addon_lock_package_comparison_key};
use crate::core::archive_io::PortableArchivePathSet;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

pub(in crate::core::bundle) fn add_bundle_addon_sources_to_zip(
    zip: &mut ZipWriter<std::fs::File>,
    installation: &DetectedFlavorInstallation,
    packages: &[AddonLockPackage],
    archive_outputs: &mut PortableArchivePathSet,
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
        add_path_to_zip(
            zip,
            &source_archive_path,
            &bundle_entry_path,
            archive_outputs,
        )?;

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
    let mut archive_outputs = PortableArchivePathSet::new();
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
        archived_files += add_path_to_zip(
            &mut zip,
            &source,
            Path::new(addon_directory),
            &mut archive_outputs,
        )?;
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
