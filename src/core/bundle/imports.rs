pub(in crate::core::bundle) use std::fs;

pub(in crate::core::bundle) use super::addon_source_archive::{
    add_bundle_addon_sources_to_zip, read_generated_addon_lock, resolve_addon_index_paths,
};
pub(in crate::core::bundle) use super::apply_model::{
    PlannedCleanup, PlannedEntry, PreparedApplyOperation, PreparedApplySource, PreparedBundleApply,
    PreviewOperation,
};
pub(in crate::core::bundle) use super::apply_policy::{
    apply_action_order, apply_group_order, build_cleanup_operations, cleanup_scope_for_entry,
    resource_policy_for_group,
};
pub(in crate::core::bundle) use super::apply_source::ApplySourceReader;
pub(in crate::core::bundle) use super::archive_read::{
    count_bundle_entries, read_manifest_from_archive,
};
pub(in crate::core::bundle) use super::character_mapping::build_character_mappings;
pub(in crate::core::bundle) use super::entry_plan::plan_extractable_entries;
pub(in crate::core::bundle) use super::execution::file_contents_equal_to_bytes;
pub(in crate::core::bundle) use super::shared::{
    BundleAddonSourceEntry, BundleAddonSourceIndex, resolve_zip_style_path, safe_file_part,
    safe_zip_segments, should_skip_path, to_zip_path, validate_plain_name, zip_dir_options,
    zip_file_options,
};
pub(in crate::core::bundle) use super::target_accounts::{
    resolve_selected_target_accounts, validate_target_compatibility,
};
pub(in crate::core::bundle) use super::wtf_archive::{
    add_character_wtf_to_zip, add_common_wtf_to_zip, resolve_character_account,
};
pub(in crate::core::bundle) use super::zip_write::{add_path_to_zip, write_toml_to_zip};
pub(in crate::core::bundle) use crate::core::addon::lock::{
    AddonLock, AddonLockPackage, addon_lock_package_comparison_key, write_addon_lock,
};
pub(in crate::core::bundle) use crate::core::backup::restore_backup;
pub(in crate::core::bundle) use crate::core::error::{AppError, AppResult};
pub(in crate::core::bundle) use crate::core::install::{
    DetectedFlavorInstallation, LocalWowAccount, discover_local_accounts,
};
pub(in crate::core::bundle) use crate::core::lua_patch::{
    CharacterMapping, LuaRewriteOptions, preview_lua_bytes_rewrite,
};
pub(in crate::core::bundle) use crate::core::manifest::{
    BundleManifest, CharacterResource, ResourceApplyPolicy,
};
