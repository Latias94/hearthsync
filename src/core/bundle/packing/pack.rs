use std::fs::{self, File};

use zip::ZipWriter;

use super::super::types::archive::{CreatedBundle, PackBundleRequest};
use super::output::{default_bundle_output_base_dir, now_rfc3339, resolve_bundle_output_path};
use super::resources::{
    add_addon_indexes_to_zip, add_addons_to_zip, add_fonts_to_zip, add_interface_assets_to_zip,
    add_optional_addon_lock_to_zip, add_wtf_characters_to_zip, add_wtf_common_to_zip_if_enabled,
    write_manifest_to_zip,
};
use crate::core::archive_io::PortableArchivePathSet;
use crate::core::error::{AppError, AppResult};

pub fn pack_bundle(mut request: PackBundleRequest) -> AppResult<CreatedBundle> {
    validate_pack_request(&request)?;
    stamp_manifest_source(&mut request)?;

    let archive_path = prepare_archive_path(&request)?;
    let mut zip = ZipWriter::new(File::create(&archive_path)?);
    let mut archive_outputs = PortableArchivePathSet::new();
    let archived_files = archive_bundle_contents(&mut zip, &mut request, &mut archive_outputs)?;
    zip.finish()?;

    Ok(CreatedBundle {
        archive_path,
        archived_files,
        manifest: request.manifest,
    })
}

fn validate_pack_request(request: &PackBundleRequest) -> AppResult<()> {
    request.manifest.validate()?;

    if request.manifest.source.flavor != request.installation.flavor {
        return Err(AppError::Validation(format!(
            "manifest source flavor `{}` does not match installation flavor `{}`",
            request.manifest.source.flavor.as_str(),
            request.installation.flavor.as_str()
        )));
    }

    Ok(())
}

fn stamp_manifest_source(request: &mut PackBundleRequest) -> AppResult<()> {
    let timestamp = now_rfc3339()?;
    request.manifest.source.exported_at = Some(timestamp);
    request.manifest.source.platform = Some(request.installation.platform);
    Ok(())
}

fn prepare_archive_path(request: &PackBundleRequest) -> AppResult<std::path::PathBuf> {
    let timestamp = request
        .manifest
        .source
        .exported_at
        .as_deref()
        .ok_or_else(|| AppError::Validation("bundle timestamp was not initialized".to_string()))?;
    let default_output_base_dir =
        default_bundle_output_base_dir(&request.installation, request.manifest_base_dir.as_deref());
    let archive_path = resolve_bundle_output_path(
        request.output_path.as_deref(),
        &request.manifest,
        timestamp,
        &default_output_base_dir,
    )?;

    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)?;
    }

    Ok(archive_path)
}

fn archive_bundle_contents(
    zip: &mut ZipWriter<File>,
    request: &mut PackBundleRequest,
    archive_outputs: &mut PortableArchivePathSet,
) -> AppResult<usize> {
    let mut archived_files = 0usize;

    archived_files += add_addons_to_zip(
        zip,
        &request.installation.addon_dir,
        &request.manifest.resources.addons,
        archive_outputs,
    )?;

    if request.manifest.resources.addon_lock {
        archived_files += add_optional_addon_lock_to_zip(
            zip,
            &request.installation,
            request.addon_state_storage_kind,
            archive_outputs,
        )?;
    }

    archived_files += add_addon_indexes_to_zip(
        zip,
        &request.manifest.resources.addon_indexes,
        request.manifest_base_dir.as_deref(),
        archive_outputs,
    )?;

    archived_files += add_wtf_common_to_zip_if_enabled(
        zip,
        &request.installation.wtf_dir,
        request.manifest.resources.wtf_common,
        archive_outputs,
    )?;
    archived_files += add_wtf_characters_to_zip(
        zip,
        &request.installation.wtf_dir,
        &mut request.manifest.resources.wtf_characters,
        archive_outputs,
    )?;
    archived_files += write_manifest_to_zip(zip, &request.manifest, archive_outputs)?;
    archived_files += add_fonts_to_zip(
        zip,
        &request.installation.fonts_dir,
        request.manifest.resources.fonts,
        archive_outputs,
    )?;
    archived_files += add_interface_assets_to_zip(
        zip,
        &request.installation.interface_dir,
        &request.manifest.resources.interface_assets,
        archive_outputs,
    )?;

    Ok(archived_files)
}
