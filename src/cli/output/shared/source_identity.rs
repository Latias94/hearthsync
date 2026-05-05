use crate::core::app::{ConfigSourceIdentityResult, ExternalPackageSourceIdentityResult};

pub(in crate::cli::output) fn format_external_package_source_identities(
    identities: &ExternalPackageSourceIdentityResult,
) -> String {
    format_source_identities(
        &identities.source_accounts,
        identities
            .source_characters
            .iter()
            .map(|character| SourceCharacterDisplay {
                source_account: character.source_account.as_deref(),
                source_server: &character.source_server,
                source_character: &character.source_character,
            }),
        identities.entries_with_source_account,
        identities.entries_with_source_character,
    )
}

pub(in crate::cli::output) fn format_config_source_identities(
    identities: &ConfigSourceIdentityResult,
) -> String {
    format_source_identities(
        &identities.source_accounts,
        identities
            .source_characters
            .iter()
            .map(|character| SourceCharacterDisplay {
                source_account: character.source_account.as_deref(),
                source_server: &character.source_server,
                source_character: &character.source_character,
            }),
        identities.entries_with_source_account,
        identities.entries_with_source_character,
    )
}

struct SourceCharacterDisplay<'a> {
    source_account: Option<&'a str>,
    source_server: &'a str,
    source_character: &'a str,
}

fn format_source_identities<'a>(
    source_accounts: &[String],
    source_characters: impl IntoIterator<Item = SourceCharacterDisplay<'a>>,
    entries_with_source_account: usize,
    entries_with_source_character: usize,
) -> String {
    if source_accounts.is_empty() && entries_with_source_character == 0 {
        return "none".to_string();
    }

    let accounts = if source_accounts.is_empty() {
        "none".to_string()
    } else {
        source_accounts.join(", ")
    };
    let characters = source_characters
        .into_iter()
        .map(format_source_character)
        .collect::<Vec<_>>();
    let characters = if characters.is_empty() {
        "none".to_string()
    } else {
        characters.join(", ")
    };

    format!(
        "accounts: {} (entries: {}), characters: {} (entries: {})",
        accounts, entries_with_source_account, characters, entries_with_source_character
    )
}

fn format_source_character(character: SourceCharacterDisplay<'_>) -> String {
    match character.source_account {
        Some(source_account) => format!(
            "{}/{}/{}",
            source_account, character.source_server, character.source_character
        ),
        None => format!("{}/{}", character.source_server, character.source_character),
    }
}
