use serde::Serialize;

use super::policy::LuaRewriteCapabilities;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterMapping {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: String,
    pub target_server: String,
    pub target_character: String,
}

impl CharacterMapping {
    pub fn source_profile_key(&self) -> String {
        format!("{} - {}", self.source_character, self.source_server)
    }

    pub fn target_profile_key(&self) -> String {
        format!("{} - {}", self.target_character, self.target_server)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LuaRewriteOptions {
    pub rewrite_profile_keys: bool,
    pub rewrite_identity_strings: bool,
}

impl LuaRewriteOptions {
    pub(super) fn limit_to(self, capabilities: LuaRewriteCapabilities) -> Self {
        Self {
            rewrite_profile_keys: self.rewrite_profile_keys && capabilities.rewrite_profile_keys,
            rewrite_identity_strings: self.rewrite_identity_strings
                && capabilities.rewrite_identity_strings,
        }
    }

    pub(super) fn is_disabled(self) -> bool {
        !self.rewrite_profile_keys && !self.rewrite_identity_strings
    }
}
