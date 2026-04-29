use std::fs::File;
use std::io::Read;

use zip::ZipArchive;

use super::super::constants::MANIFEST_ENTRY;
use super::super::types::archive::BundleEntryCounts;
use super::safety::reject_unsupported_bundle_symlink_entry;
use crate::core::error::AppResult;
use crate::core::manifest::BundleManifest;

pub(in crate::core::bundle) fn read_manifest_from_archive(
    archive: &mut ZipArchive<File>,
) -> AppResult<BundleManifest> {
    let mut manifest_file = archive.by_name(MANIFEST_ENTRY)?;
    reject_unsupported_bundle_symlink_entry(
        manifest_file.name(),
        manifest_file.is_symlink(),
        manifest_file.is_dir(),
    )?;
    let mut content = String::new();
    manifest_file.read_to_string(&mut content)?;
    let manifest = toml::from_str::<BundleManifest>(&content)?;
    manifest.validate()?;
    Ok(manifest)
}

pub(in crate::core::bundle) fn count_bundle_entries(
    archive: &mut ZipArchive<File>,
) -> AppResult<BundleEntryCounts> {
    let mut counts = BundleEntryCounts::default();

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        reject_unsupported_bundle_symlink_entry(file.name(), file.is_symlink(), file.is_dir())?;
        if file.is_dir() {
            continue;
        }

        counts.total_files += 1;
        let name = file.name();
        if name == MANIFEST_ENTRY || name.starts_with("metadata/") {
            counts.metadata += 1;
        } else if name.starts_with("addons/") {
            counts.addons += 1;
        } else if name.starts_with("wtf/common/") {
            counts.wtf_common += 1;
        } else if name.starts_with("wtf/characters/") {
            counts.wtf_characters += 1;
        } else if name.starts_with("fonts/") {
            counts.fonts += 1;
        } else if name.starts_with("interface/") {
            counts.interface_assets += 1;
        }
    }

    Ok(counts)
}
