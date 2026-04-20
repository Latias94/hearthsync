mod character;
mod common;
mod context;

use std::collections::BTreeMap;

use super::*;

pub(in crate::core::bundle) use context::plan_extractable_entries;

struct EntryPlanningContext<'a> {
    installation: &'a DetectedFlavorInstallation,
    manifest: &'a BundleManifest,
    character_mappings: &'a [CharacterMapping],
    common_account_targets: &'a BTreeMap<String, String>,
    default_target_account: Option<&'a str>,
    selected_target_accounts: &'a [String],
}
