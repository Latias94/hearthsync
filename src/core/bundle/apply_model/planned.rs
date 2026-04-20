use std::path::PathBuf;

use crate::core::lua_patch::CharacterMapping;

use super::super::{ApplyGroup, WtfScope};

#[derive(Debug, Clone)]
pub(in crate::core::bundle) struct PlannedEntry {
    pub(in crate::core::bundle) archive_name: String,
    pub(in crate::core::bundle) destination: PathBuf,
    pub(in crate::core::bundle) rewrites: Vec<CharacterMapping>,
    pub(in crate::core::bundle) group: ApplyGroup,
    pub(in crate::core::bundle) wtf_scope: Option<WtfScope>,
    pub(in crate::core::bundle) target_account: Option<String>,
    pub(in crate::core::bundle) target_server: Option<String>,
    pub(in crate::core::bundle) target_character: Option<String>,
}

#[derive(Debug, Clone)]
pub(in crate::core::bundle) struct PlannedCleanup {
    pub(in crate::core::bundle) group: ApplyGroup,
    pub(in crate::core::bundle) destination: PathBuf,
    pub(in crate::core::bundle) target_account: Option<String>,
    pub(in crate::core::bundle) target_server: Option<String>,
    pub(in crate::core::bundle) target_character: Option<String>,
}
