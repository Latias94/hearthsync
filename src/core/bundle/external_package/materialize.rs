use std::fs::{self, File};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::types::{ExternalPackageAnalysis, ExternalPackageEntry, ExternalPackageSourceKind};
use crate::core::archive_io::copy_reader_to_path;
use crate::core::bundle::shared::{join_segments, safe_zip_segments};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{DetectedFlavorInstallation, HostPlatform, WowFlavor};

pub(super) fn create_staging_installation(
    stage_root: &Path,
    flavor: WowFlavor,
    platform: HostPlatform,
) -> AppResult<DetectedFlavorInstallation> {
    let product_root = stage_root.join("World of Warcraft");
    let flavor_root = product_root.join(flavor.folder_name());
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    fs::create_dir_all(&addon_dir)?;
    fs::create_dir_all(&wtf_dir)?;
    fs::create_dir_all(&fonts_dir)?;

    Ok(DetectedFlavorInstallation {
        platform,
        product_root,
        flavor_root,
        flavor,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    })
}

pub(super) fn materialize_analysis_to_installation(
    analysis: &ExternalPackageAnalysis,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    match analysis.source_kind {
        ExternalPackageSourceKind::Directory => {
            for entry in &analysis.entries {
                let source_path =
                    resolve_directory_source_entry_path(&analysis.source_path, entry)?;
                let destination = destination_path_for_normalized_entry(entry, installation)?;
                copy_file_to_destination(&source_path, &destination)?;
            }
        }
        ExternalPackageSourceKind::ZipArchive => {
            let file = File::open(&analysis.source_path)?;
            let mut archive = ZipArchive::new(file)?;
            for entry in &analysis.entries {
                let destination = destination_path_for_normalized_entry(entry, installation)?;
                write_zip_entry_to_destination(&mut archive, &entry.source_path, &destination)?;
            }
        }
    }

    Ok(())
}

fn destination_path_for_normalized_entry(
    entry: &ExternalPackageEntry,
    installation: &DetectedFlavorInstallation,
) -> AppResult<PathBuf> {
    let segments = safe_zip_segments(&entry.normalized_path)?;
    let destination = match segments.as_slice() {
        ["addons", rest @ ..] if !rest.is_empty() => join_segments(&installation.addon_dir, rest),
        ["wtf", "common", "Config.wtf"] => installation.wtf_dir.join("Config.wtf"),
        ["wtf", "common", "root", "SavedVariables", rest @ ..] if !rest.is_empty() => installation
            .wtf_dir
            .join("Account")
            .join("SavedVariables")
            .join(join_segments(Path::new(""), rest)),
        ["wtf", "common", "accounts", account, rest @ ..] if !rest.is_empty() => installation
            .wtf_dir
            .join("Account")
            .join(account)
            .join(join_segments(Path::new(""), rest)),
        ["wtf", "characters", account, server, character, rest @ ..] if !rest.is_empty() => {
            installation
                .wtf_dir
                .join("Account")
                .join(account)
                .join(server)
                .join(character)
                .join(join_segments(Path::new(""), rest))
        }
        ["fonts", rest @ ..] if !rest.is_empty() => join_segments(&installation.fonts_dir, rest),
        ["interface", rest @ ..] if !rest.is_empty() => {
            join_segments(&installation.interface_dir, rest)
        }
        _ => {
            return Err(AppError::Validation(format!(
                "unsupported normalized external package path: {}",
                entry.normalized_path
            )));
        }
    };

    Ok(destination)
}

fn copy_file_to_destination(source_path: &Path, destination: &Path) -> AppResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source_path, destination)?;
    Ok(())
}

fn write_zip_entry_to_destination(
    archive: &mut ZipArchive<File>,
    source_entry_name: &str,
    destination: &Path,
) -> AppResult<()> {
    let mut entry = archive.by_name(source_entry_name).map_err(|_| {
        AppError::NotFound(format!(
            "external package entry is missing during normalization: {source_entry_name}"
        ))
    })?;
    copy_reader_to_path(&mut entry, destination)
}

fn resolve_directory_source_entry_path(
    root: &Path,
    entry: &ExternalPackageEntry,
) -> AppResult<PathBuf> {
    let segments = safe_zip_segments(&entry.source_path)?;
    Ok(join_segments(root, &segments))
}
