mod index_paths;
mod lock;
mod source_bundle;

pub(in crate::core::bundle) use index_paths::resolve_addon_index_paths;
pub(in crate::core::bundle) use lock::read_generated_addon_lock;
pub(in crate::core::bundle) use source_bundle::add_bundle_addon_sources_to_zip;
