use std::fs;
use std::path::Path;

use super::super::*;

pub(in crate::core::bundle) fn resolve_character_account(
    wtf_dir: &Path,
    character: &CharacterResource,
) -> AppResult<String> {
    if let Some(account) = &character.source_account {
        validate_plain_name("account", account)?;
        return Ok(account.clone());
    }

    let mut matches = Vec::new();
    let account_root = wtf_dir.join("Account");
    if !account_root.exists() {
        return Err(AppError::NotFound(format!(
            "account root does not exist: {}",
            account_root.display()
        )));
    }

    for entry in fs::read_dir(account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let candidate = account_dir
            .join(&character.source_server)
            .join(&character.source_character);
        if candidate.exists() {
            matches.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    match matches.as_slice() {
        [account] => Ok(account.clone()),
        [] => Err(AppError::NotFound(format!(
            "no account contains character `{}` on server `{}`",
            character.source_character, character.source_server
        ))),
        many => Err(AppError::Validation(format!(
            "character `{}` on server `{}` exists in multiple accounts: {:?}. Set source_account explicitly.",
            character.source_character, character.source_server, many
        ))),
    }
}
