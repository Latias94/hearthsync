use super::super::shared::path::validate_plain_name;
use super::super::types::apply::BundleApplyMappings;
use crate::core::error::{AppError, AppResult};
use crate::core::install::LocalWowAccount;
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{BundleManifest, CharacterMappingMode};

pub(in crate::core::bundle) fn resolve_selected_target_accounts(
    manifest: &BundleManifest,
    discovered_accounts: &[LocalWowAccount],
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
) -> AppResult<Vec<String>> {
    if !manifest.resources.wtf_common {
        return Ok(Vec::new());
    }

    if apply_mappings.all_accounts {
        return Ok(discovered_accounts
            .iter()
            .map(|account| account.account_name.clone())
            .collect());
    }

    if !apply_mappings.selected_accounts.is_empty() {
        let mut selected = apply_mappings.selected_accounts.clone();
        selected.sort();
        selected.dedup();
        for account in &selected {
            validate_plain_name("selected account", account)?;
        }
        return Ok(selected);
    }

    if manifest.mapping.character_mode != CharacterMappingMode::KeepOriginal
        && let Some(target_account) = &apply_mappings.target_account
    {
        validate_plain_name("target account", target_account)?;
        return Ok(vec![target_account.clone()]);
    }

    let mut mapped_accounts = character_mappings
        .iter()
        .map(|mapping| mapping.target_account.clone())
        .collect::<Vec<_>>();
    mapped_accounts.sort();
    mapped_accounts.dedup();

    if mapped_accounts.len() == 1 {
        return Ok(mapped_accounts);
    }

    Err(AppError::Validation(
        "common WTF resources require explicit target account selection. Use `--select-account`, `--all-accounts`, or `--target-account`.".to_string(),
    ))
}
