mod character;
mod common;
pub(super) mod context;

use std::collections::BTreeMap;

use crate::core::install::DetectedFlavorInstallation;
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::BundleManifest;

struct EntryPlanningContext<'a> {
    installation: &'a DetectedFlavorInstallation,
    manifest: &'a BundleManifest,
    character_mappings: &'a [CharacterMapping],
    common_account_targets: &'a BTreeMap<String, String>,
    default_target_account: Option<&'a str>,
    selected_target_accounts: &'a [String],
}
