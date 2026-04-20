pub(in crate::core::bundle) use std::fs;

pub(in crate::core::bundle) use super::addon_source_archive::{
    add_bundle_addon_sources_to_zip, read_generated_addon_lock, resolve_addon_index_paths,
};
pub(in crate::core::bundle) use super::apply_model::{
    PlannedCleanup, PlannedEntry, PreparedApplyOperation, PreparedApplySource, PreparedBundleApply,
    PreviewOperation,
};
pub(in crate::core::bundle) use super::archive_read::{
    count_bundle_entries, read_manifest_from_archive,
};
pub(in crate::core::bundle) use super::shared::{
    BundleAddonSourceEntry, BundleAddonSourceIndex, safe_file_part, safe_zip_segments,
    should_skip_path, to_zip_path, validate_plain_name, zip_dir_options, zip_file_options,
};
pub(in crate::core::bundle) use super::wtf_archive::{
    add_character_wtf_to_zip, add_common_wtf_to_zip, resolve_character_account,
};
pub(in crate::core::bundle) use super::zip_write::{add_path_to_zip, write_toml_to_zip};
pub(in crate::core::bundle) use crate::core::addon::lock::{
    AddonLock, AddonLockPackage, addon_lock_package_comparison_key, write_addon_lock,
};
pub(in crate::core::bundle) use crate::core::error::{AppError, AppResult};
pub(in crate::core::bundle) use crate::core::install::{
    DetectedFlavorInstallation, LocalWowAccount,
};
pub(in crate::core::bundle) use crate::core::lua_patch::CharacterMapping;
pub(in crate::core::bundle) use crate::core::manifest::{BundleManifest, CharacterResource};
