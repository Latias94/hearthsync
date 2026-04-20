use std::fs::File;
use std::path::Path;

use zip::ZipWriter;

use super::super::shared::validate_plain_name;
use super::super::zip_write::add_path_to_zip;
use crate::core::error::{AppError, AppResult};
use crate::core::manifest::CharacterResource;

pub(in crate::core::bundle) fn add_character_wtf_to_zip(
    zip: &mut ZipWriter<File>,
    wtf_dir: &Path,
    character: &CharacterResource,
    account: &str,
) -> AppResult<usize> {
    validate_plain_name("server", &character.source_server)?;
    validate_plain_name("character", &character.source_character)?;
    validate_plain_name("account", account)?;
    let character_dir = wtf_dir
        .join("Account")
        .join(account)
        .join(&character.source_server)
        .join(&character.source_character);

    if !character_dir.exists() {
        return Err(AppError::NotFound(format!(
            "character WTF directory does not exist: {}",
            character_dir.display()
        )));
    }

    add_path_to_zip(
        zip,
        &character_dir,
        &Path::new("wtf/characters")
            .join(account)
            .join(&character.source_server)
            .join(&character.source_character),
    )
}
