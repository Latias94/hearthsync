pub(in crate::core::bundle) use std::fs;

pub(in crate::core::bundle) use super::addon_source_archive::{
    add_bundle_addon_sources_to_zip, read_generated_addon_lock, resolve_addon_index_paths,
};
pub(in crate::core::bundle) use super::apply_model::{PreparedApplySource, PreparedBundleApply};
pub(in crate::core::bundle) use super::shared::{
    BundleAddonSourceEntry, BundleAddonSourceIndex, safe_file_part, validate_plain_name,
    zip_file_options,
};
pub(in crate::core::bundle) use super::wtf_archive::{
    add_character_wtf_to_zip, add_common_wtf_to_zip, resolve_character_account,
};
pub(in crate::core::bundle) use super::zip_write::{add_path_to_zip, write_toml_to_zip};
pub(in crate::core::bundle) use crate::core::addon::lock::{
    AddonLock, AddonLockPackage, addon_lock_package_comparison_key, write_addon_lock,
};
pub(in crate::core::bundle) use crate::core::error::{AppError, AppResult};
pub(in crate::core::bundle) use crate::core::install::DetectedFlavorInstallation;
pub(in crate::core::bundle) use crate::core::manifest::{BundleManifest, CharacterResource};
