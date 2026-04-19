use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use zip::{ZipArchive, ZipWriter};

use super::*;
use crate::core::error::{AppError, AppResult};

pub fn pack_bundle(mut request: PackBundleRequest) -> AppResult<CreatedBundle> {
    request.manifest.validate()?;

    if request.manifest.source.flavor != request.installation.flavor {
        return Err(AppError::Validation(format!(
            "manifest source flavor `{}` does not match installation flavor `{}`",
            request.manifest.source.flavor.as_str(),
            request.installation.flavor.as_str()
        )));
    }

    let timestamp = now_rfc3339()?;
    request.manifest.source.exported_at = Some(timestamp.clone());
    request.manifest.source.platform = Some(request.installation.platform);

    let default_output_base_dir =
        default_bundle_output_base_dir(&request.installation, request.manifest_base_dir.as_deref());
    let archive_path = resolve_bundle_output_path(
        request.output_path.as_deref(),
        &request.manifest,
        &timestamp,
        &default_output_base_dir,
    )?;
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(&archive_path)?;
    let mut zip = ZipWriter::new(file);
    let mut archived_files = 0usize;

    for addon in &request.manifest.resources.addons {
        validate_plain_name("addon", addon)?;
        let source = request.installation.addon_dir.join(addon);
        if !source.exists() {
            return Err(AppError::NotFound(format!(
                "addon does not exist: {}",
                source.display()
            )));
        }
        archived_files += add_path_to_zip(&mut zip, &source, &Path::new("addons").join(addon))?;
    }

    if request.manifest.resources.addon_lock {
        let lock_result = write_addon_lock(&request.installation)?;
        if lock_result.removed {
            return Err(AppError::Validation(
                "cannot embed addon lock because no tracked addon packages were found".to_string(),
            ));
        }
        let lock = read_generated_addon_lock(&lock_result.lock_path)?;
        let source_index =
            add_bundle_addon_sources_to_zip(&mut zip, &request.installation, &lock.packages)?;
        archived_files += source_index.sources.len();
        archived_files += write_toml_to_zip(&mut zip, ADDON_SOURCE_INDEX_ENTRY, &source_index)?;
        archived_files += write_toml_to_zip(&mut zip, ADDON_LOCK_ENTRY, &lock)?;
    }

    let addon_index_paths = resolve_addon_index_paths(
        &request.manifest.resources.addon_indexes,
        request.manifest_base_dir.as_deref(),
    )?;
    for (file_name, source_path) in addon_index_paths {
        archived_files += add_path_to_zip(
            &mut zip,
            &source_path,
            &Path::new(ADDON_INDEX_ENTRY_ROOT).join(file_name),
        )?;
    }

    if request.manifest.resources.wtf_common {
        archived_files += add_common_wtf_to_zip(&mut zip, &request.installation.wtf_dir)?;
    }

    for character in &mut request.manifest.resources.wtf_characters {
        let resolved_account = resolve_character_account(&request.installation.wtf_dir, character)?;
        character.source_account = Some(resolved_account.clone());
        archived_files += add_character_wtf_to_zip(
            &mut zip,
            &request.installation.wtf_dir,
            character,
            &resolved_account,
        )?;
    }

    zip.start_file(MANIFEST_ENTRY, zip_file_options())?;
    zip.write_all(toml::to_string_pretty(&request.manifest)?.as_bytes())?;
    archived_files += 1;

    if request.manifest.resources.fonts {
        archived_files += add_path_to_zip(
            &mut zip,
            &request.installation.fonts_dir,
            Path::new("fonts"),
        )?;
    }

    for asset in &request.manifest.resources.interface_assets {
        validate_plain_name("interface asset", asset)?;
        let source = request.installation.interface_dir.join(asset);
        if !source.exists() {
            return Err(AppError::NotFound(format!(
                "interface asset does not exist: {}",
                source.display()
            )));
        }
        archived_files += add_path_to_zip(&mut zip, &source, &Path::new("interface").join(asset))?;
    }

    zip.finish()?;

    Ok(CreatedBundle {
        archive_path,
        archived_files,
        manifest: request.manifest,
    })
}

pub fn inspect_bundle(path: &Path) -> AppResult<BundleInspection> {
    let archive_path = path.to_path_buf();
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let manifest = read_manifest_from_archive(&mut archive)?;
    manifest.validate()?;
    let entries = count_bundle_entries(&mut archive)?;

    Ok(BundleInspection {
        archive_path,
        manifest,
        entries,
    })
}

pub fn load_apply_mappings(path: &Path) -> AppResult<BundleApplyMappings> {
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn resolve_bundle_output_path(
    output_path: Option<&Path>,
    manifest: &BundleManifest,
    timestamp: &str,
    default_base_dir: &Path,
) -> AppResult<PathBuf> {
    let file_name = format!(
        "bundle-{}-{}.zip",
        safe_file_part(&manifest.package.id),
        compact_timestamp(timestamp)
    );

    match output_path {
        Some(path) if path.extension().is_some_and(|extension| extension == "zip") => {
            Ok(resolve_output_reference(path, default_base_dir))
        }
        Some(path) => Ok(resolve_output_reference(path, default_base_dir).join(file_name)),
        None => Ok(default_base_dir.join(file_name)),
    }
}

fn default_bundle_output_base_dir(
    installation: &DetectedFlavorInstallation,
    manifest_base_dir: Option<&Path>,
) -> PathBuf {
    manifest_base_dir
        .map(Path::to_path_buf)
        .or_else(|| installation.product_root.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| installation.product_root.clone())
}

fn resolve_output_reference(path: &Path, default_base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        default_base_dir.join(path)
    }
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}

fn compact_timestamp(timestamp: &str) -> String {
    timestamp
        .chars()
        .filter(|char| char.is_ascii_alphanumeric())
        .collect::<String>()
}
