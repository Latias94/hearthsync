use std::fs;
use std::path::Path;

use crate::core::error::AppResult;

use super::model::{DetectedFlavorInstallation, LocalWowAccount, LocalWowCharacter};

pub fn discover_local_accounts(
    installation: &DetectedFlavorInstallation,
) -> AppResult<Vec<LocalWowAccount>> {
    let account_root = installation.wtf_dir.join("Account");
    if !account_root.exists() {
        return Ok(Vec::new());
    }

    let mut accounts = Vec::new();
    for entry in fs::read_dir(&account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let account_name = entry.file_name().to_string_lossy().to_string();
        if is_reserved_account_entry(&account_name) {
            continue;
        }

        let saved_variables_dir = account_dir.join("SavedVariables");
        let mut characters = discover_account_characters(&account_dir)?;
        if !saved_variables_dir.is_dir() && characters.is_empty() {
            continue;
        }

        characters.sort_by(|left, right| {
            left.server
                .cmp(&right.server)
                .then(left.character.cmp(&right.character))
        });

        accounts.push(LocalWowAccount {
            account_name,
            account_dir,
            saved_variables_dir,
            characters,
        });
    }

    accounts.sort_by(|left, right| left.account_name.cmp(&right.account_name));
    Ok(accounts)
}

fn is_reserved_account_entry(name: &str) -> bool {
    name.eq_ignore_ascii_case("SavedVariables")
}

fn discover_account_characters(account_dir: &Path) -> AppResult<Vec<LocalWowCharacter>> {
    let mut characters = Vec::new();

    for server_entry in fs::read_dir(account_dir)? {
        let server_entry = server_entry?;
        let server_dir = server_entry.path();
        if !server_dir.is_dir() {
            continue;
        }

        let server_name = server_entry.file_name().to_string_lossy().to_string();
        if server_name.eq_ignore_ascii_case("SavedVariables") {
            continue;
        }

        for character_entry in fs::read_dir(&server_dir)? {
            let character_entry = character_entry?;
            let character_dir = character_entry.path();
            if !looks_like_character_dir(&character_dir)? {
                continue;
            }

            characters.push(LocalWowCharacter {
                server: server_name.clone(),
                character: character_entry.file_name().to_string_lossy().to_string(),
                character_dir,
            });
        }
    }

    Ok(characters)
}

fn looks_like_character_dir(character_dir: &Path) -> AppResult<bool> {
    if !character_dir.is_dir() {
        return Ok(false);
    }

    let saved_variables_dir = character_dir.join("SavedVariables");
    if saved_variables_dir.is_dir() {
        return Ok(true);
    }

    for entry in fs::read_dir(character_dir)? {
        let entry = entry?;
        if entry.path().is_file() {
            return Ok(true);
        }
    }

    Ok(false)
}
