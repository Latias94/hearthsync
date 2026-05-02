use crate::core::app::{
    BundleCharacterResourceResult, CharacterMappingResult, LocalWowAccountResult,
};

use super::lists::format_string_list_or_none;

pub(in crate::cli::output) fn format_bundle_characters(
    resources: &[BundleCharacterResourceResult],
) -> String {
    let characters = resources
        .iter()
        .map(|character| {
            format!(
                "{}/{}/{}",
                character
                    .source_account
                    .as_deref()
                    .unwrap_or("<unknown-account>"),
                character.source_server,
                character.source_character
            )
        })
        .collect::<Vec<_>>();

    if characters.is_empty() {
        "none".to_string()
    } else {
        characters.join(", ")
    }
}

pub(in crate::cli::output) fn format_discovered_accounts(
    accounts: &[LocalWowAccountResult],
) -> String {
    if accounts.is_empty() {
        "none".to_string()
    } else {
        accounts
            .iter()
            .map(|account| {
                format!(
                    "{}({} chars)",
                    account.account_name,
                    account.characters.len()
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(in crate::cli::output) fn format_selected_accounts(accounts: &[String]) -> String {
    format_string_list_or_none(accounts)
}

pub(in crate::cli::output) fn format_character_mapping_summary(
    mappings: &[CharacterMappingResult],
) -> String {
    if mappings.is_empty() {
        "none".to_string()
    } else {
        format_character_mappings(mappings)
    }
}

pub(in crate::cli::output) fn format_character_mappings(
    mappings: &[CharacterMappingResult],
) -> String {
    mappings
        .iter()
        .map(|mapping| {
            format!(
                "{}/{}/{} -> {}/{}/{}",
                mapping
                    .source_account
                    .as_deref()
                    .unwrap_or("<unknown-account>"),
                mapping.source_server,
                mapping.source_character,
                mapping.target_account,
                mapping.target_server,
                mapping.target_character
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
