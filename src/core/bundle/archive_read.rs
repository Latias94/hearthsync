mod addon_lock;
mod entries;
mod inspect;

pub(in crate::core::bundle) use addon_lock::extract_embedded_addon_lock;
pub(in crate::core::bundle) use entries::{
    collect_bundle_entry_names, extract_archive_entry_to_path, read_bundle_entry_bytes_from_archive,
};
pub(in crate::core::bundle) use inspect::{count_bundle_entries, read_manifest_from_archive};
