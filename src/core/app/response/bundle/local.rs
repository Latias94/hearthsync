use std::path::PathBuf;

use serde::Serialize;

use crate::core::install::{LocalWowAccount, LocalWowCharacter};
use crate::core::lua_patch::CharacterMapping;

use super::super::super::map_owned_vec;

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowCharacterResult {
    pub server: String,
    pub character: String,
    pub character_dir: PathBuf,
}

impl LocalWowCharacterResult {
    pub(crate) fn from_domain(value: LocalWowCharacter) -> Self {
        Self {
            server: value.server,
            character: value.character,
            character_dir: value.character_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalWowAccountResult {
    pub account_name: String,
    pub account_dir: PathBuf,
    pub saved_variables_dir: PathBuf,
    pub characters: Vec<LocalWowCharacterResult>,
}

impl LocalWowAccountResult {
    pub(crate) fn from_domain(value: LocalWowAccount) -> Self {
        Self {
            account_name: value.account_name,
            account_dir: value.account_dir,
            saved_variables_dir: value.saved_variables_dir,
            characters: map_owned_vec(value.characters, LocalWowCharacterResult::from_domain),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CharacterMappingResult {
    pub source_account: Option<String>,
    pub source_server: String,
    pub source_character: String,
    pub target_account: String,
    pub target_server: String,
    pub target_character: String,
}

impl CharacterMappingResult {
    pub(crate) fn from_domain(value: CharacterMapping) -> Self {
        Self {
            source_account: value.source_account,
            source_server: value.source_server,
            source_character: value.source_character,
            target_account: value.target_account,
            target_server: value.target_server,
            target_character: value.target_character,
        }
    }
}
