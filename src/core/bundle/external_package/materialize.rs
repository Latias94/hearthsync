use std::fs::{self, File};
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use super::types::{ExternalPackageAnalysis, ExternalPackageEntry, ExternalPackageSourceKind};
use crate::core::archive_io::copy_reader_to_path;
use crate::core::bundle::entry_layout::{BundleArchiveEntry, classify_bundle_archive_entry};
use crate::core::bundle::shared::path::{join_segments, safe_zip_segments};
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
    let Some(classified_entry) = classify_bundle_archive_entry(&entry.normalized_path)? else {
        return Err(unsupported_normalized_external_package_path(
            &entry.normalized_path,
        ));
    };

    let destination = match classified_entry {
        BundleArchiveEntry::Metadata { .. } => {
            return Err(unsupported_normalized_external_package_path(
                &entry.normalized_path,
            ));
        }
        BundleArchiveEntry::Addon { rest } => join_segments(&installation.addon_dir, &rest),
        BundleArchiveEntry::CommonConfig => installation.wtf_dir.join("Config.wtf"),
        BundleArchiveEntry::CommonRootSavedVariables { rest } => join_segments(
            &installation.wtf_dir.join("Account").join("SavedVariables"),
            &rest,
        ),
        BundleArchiveEntry::CommonAccountSavedVariables {
            source_account,
            rest,
        } => join_segments(
            &installation
                .wtf_dir
                .join("Account")
                .join(source_account)
                .join("SavedVariables"),
            &rest,
        ),
        BundleArchiveEntry::CommonAccountFile {
            source_account,
            rest,
        } => join_segments(
            &installation.wtf_dir.join("Account").join(source_account),
            &rest,
        ),
        BundleArchiveEntry::CharacterFile {
            source_account,
            server,
            character,
            rest,
        } => join_segments(
            &installation
                .wtf_dir
                .join("Account")
                .join(source_account)
                .join(server)
                .join(character),
            &rest,
        ),
        BundleArchiveEntry::Fonts { rest } => join_segments(&installation.fonts_dir, &rest),
        BundleArchiveEntry::Interface { rest } => join_segments(&installation.interface_dir, &rest),
    };

    Ok(destination)
}

fn unsupported_normalized_external_package_path(path: &str) -> AppError {
    AppError::Validation(format!(
        "unsupported normalized external package path: {path}"
    ))
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        create_staging_installation, destination_path_for_normalized_entry,
        unsupported_normalized_external_package_path,
    };
    use crate::core::bundle::ExternalPackageEntry;
    use crate::core::bundle::types::apply::{ApplyGroup, WtfScope};
    use crate::core::install::{HostPlatform, WowFlavor};

    #[test]
    fn destination_path_for_normalized_entry_maps_account_saved_variables_with_shared_layout() {
        let temp = tempdir().expect("temp dir");
        let installation =
            create_staging_installation(temp.path(), WowFlavor::Retail, HostPlatform::Windows)
                .expect("staging installation");
        let entry = sample_entry(
            "wtf/common/accounts/ACCOUNT/SavedVariables/Details.lua",
            ApplyGroup::WtfCommon,
            Some(WtfScope::AccountSavedVariables),
        );

        let destination = destination_path_for_normalized_entry(&entry, &installation)
            .expect("account saved variables destination");

        assert_eq!(
            destination,
            installation
                .wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("SavedVariables")
                .join("Details.lua")
        );
    }

    #[test]
    fn destination_path_for_normalized_entry_maps_account_files_without_saved_variables_prefix() {
        let temp = tempdir().expect("temp dir");
        let installation =
            create_staging_installation(temp.path(), WowFlavor::Retail, HostPlatform::Windows)
                .expect("staging installation");
        let entry = sample_entry(
            "wtf/common/accounts/ACCOUNT/bindings-cache.wtf",
            ApplyGroup::WtfCommon,
            Some(WtfScope::CacheLike),
        );

        let destination = destination_path_for_normalized_entry(&entry, &installation)
            .expect("account cache file destination");

        assert_eq!(
            destination,
            installation
                .wtf_dir
                .join("Account")
                .join("ACCOUNT")
                .join("bindings-cache.wtf")
        );
    }

    #[test]
    fn destination_path_for_normalized_entry_rejects_metadata_layout() {
        let temp = tempdir().expect("temp dir");
        let installation =
            create_staging_installation(temp.path(), WowFlavor::Retail, HostPlatform::Windows)
                .expect("staging installation");
        let entry = sample_entry("metadata/source/sources.toml", ApplyGroup::Metadata, None);

        let error = destination_path_for_normalized_entry(&entry, &installation)
            .expect_err("metadata layout should be rejected");

        assert_eq!(
            error.to_string(),
            unsupported_normalized_external_package_path("metadata/source/sources.toml")
                .to_string()
        );
    }

    fn sample_entry(
        normalized_path: &str,
        group: ApplyGroup,
        wtf_scope: Option<WtfScope>,
    ) -> ExternalPackageEntry {
        ExternalPackageEntry {
            source_path: normalized_path.to_string(),
            normalized_path: normalized_path.to_string(),
            group,
            wtf_scope,
            source_account: None,
            source_server: None,
            source_character: None,
        }
    }
}
