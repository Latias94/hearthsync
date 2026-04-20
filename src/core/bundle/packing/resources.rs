use std::fs::File;
use std::io::Write;
use std::path::Path;

use zip::ZipWriter;

use super::super::addon_source_archive::index_paths::resolve_addon_index_paths;
use super::super::addon_source_archive::lock::read_generated_addon_lock;
use super::super::addon_source_archive::source_bundle::add_bundle_addon_sources_to_zip;
use super::super::constants::{
    ADDON_INDEX_ENTRY_ROOT, ADDON_LOCK_ENTRY, ADDON_SOURCE_INDEX_ENTRY, MANIFEST_ENTRY,
};
use super::super::shared::path::validate_plain_name;
use super::super::shared::zip_options::zip_file_options;
use super::super::wtf_archive::character::add_character_wtf_to_zip;
use super::super::wtf_archive::common::add_common_wtf_to_zip;
use super::super::wtf_archive::resolve::resolve_character_account;
use super::super::zip_write::{add_path_to_zip, write_toml_to_zip};
use crate::core::addon::lock::write_addon_lock;
use crate::core::archive_io::start_file_to_zip;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::manifest::{BundleManifest, CharacterResource};

pub(in crate::core::bundle::packing) fn add_addons_to_zip(
    zip: &mut ZipWriter<File>,
    addon_dir: &Path,
    addons: &[String],
) -> AppResult<usize> {
    let mut archived_files = 0usize;

    for addon in addons {
        validate_plain_name("addon", addon)?;
        let source = addon_dir.join(addon);
        if !source.exists() {
            return Err(AppError::NotFound(format!(
                "addon does not exist: {}",
                source.display()
            )));
        }

        archived_files += add_path_to_zip(zip, &source, &Path::new("addons").join(addon))?;
    }

    Ok(archived_files)
}

pub(in crate::core::bundle::packing) fn add_optional_addon_lock_to_zip(
    zip: &mut ZipWriter<File>,
    installation: &DetectedFlavorInstallation,
) -> AppResult<usize> {
    let lock_result = write_addon_lock(installation)?;
    if lock_result.removed {
        return Err(AppError::Validation(
            "cannot embed addon lock because no tracked addon packages were found".to_string(),
        ));
    }

    let lock = read_generated_addon_lock(&lock_result.lock_path)?;
    let source_index = add_bundle_addon_sources_to_zip(zip, installation, &lock.packages)?;
    let mut archived_files = source_index.sources.len();
    archived_files += write_toml_to_zip(zip, ADDON_SOURCE_INDEX_ENTRY, &source_index)?;
    archived_files += write_toml_to_zip(zip, ADDON_LOCK_ENTRY, &lock)?;
    Ok(archived_files)
}

pub(in crate::core::bundle::packing) fn add_addon_indexes_to_zip(
    zip: &mut ZipWriter<File>,
    addon_indexes: &[String],
    manifest_base_dir: Option<&Path>,
) -> AppResult<usize> {
    let addon_index_paths = resolve_addon_index_paths(addon_indexes, manifest_base_dir)?;
    let mut archived_files = 0usize;

    for (file_name, source_path) in addon_index_paths {
        archived_files += add_path_to_zip(
            zip,
            &source_path,
            &Path::new(ADDON_INDEX_ENTRY_ROOT).join(file_name),
        )?;
    }

    Ok(archived_files)
}

pub(in crate::core::bundle::packing) fn add_wtf_common_to_zip_if_enabled(
    zip: &mut ZipWriter<File>,
    wtf_dir: &Path,
    enabled: bool,
) -> AppResult<usize> {
    if enabled {
        add_common_wtf_to_zip(zip, wtf_dir)
    } else {
        Ok(0)
    }
}

pub(in crate::core::bundle::packing) fn add_wtf_characters_to_zip(
    zip: &mut ZipWriter<File>,
    wtf_dir: &Path,
    characters: &mut [CharacterResource],
) -> AppResult<usize> {
    let mut archived_files = 0usize;

    for character in characters {
        let resolved_account = resolve_character_account(wtf_dir, character)?;
        character.source_account = Some(resolved_account.clone());
        archived_files += add_character_wtf_to_zip(zip, wtf_dir, character, &resolved_account)?;
    }

    Ok(archived_files)
}

pub(in crate::core::bundle::packing) fn add_fonts_to_zip(
    zip: &mut ZipWriter<File>,
    fonts_dir: &Path,
    enabled: bool,
) -> AppResult<usize> {
    if enabled {
        add_path_to_zip(zip, fonts_dir, Path::new("fonts"))
    } else {
        Ok(0)
    }
}

pub(in crate::core::bundle::packing) fn add_interface_assets_to_zip(
    zip: &mut ZipWriter<File>,
    interface_dir: &Path,
    interface_assets: &[String],
) -> AppResult<usize> {
    let mut archived_files = 0usize;

    for asset in interface_assets {
        validate_plain_name("interface asset", asset)?;
        let source = interface_dir.join(asset);
        if !source.exists() {
            return Err(AppError::NotFound(format!(
                "interface asset does not exist: {}",
                source.display()
            )));
        }

        archived_files += add_path_to_zip(zip, &source, &Path::new("interface").join(asset))?;
    }

    Ok(archived_files)
}

pub(in crate::core::bundle::packing) fn write_manifest_to_zip(
    zip: &mut ZipWriter<File>,
    manifest: &BundleManifest,
) -> AppResult<usize> {
    start_file_to_zip(zip, MANIFEST_ENTRY, zip_file_options())?;
    zip.write_all(toml::to_string_pretty(manifest)?.as_bytes())?;
    Ok(1)
}
