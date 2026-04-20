use super::{CharacterMappingOverride, CharacterResource};
use crate::core::error::{AppError, AppResult};
use crate::core::lua_patch::CharacterMapping;

pub(super) fn resolve_mapping_override<'a>(
    resource: &CharacterResource,
    overrides: &'a [CharacterMappingOverride],
) -> AppResult<Option<&'a CharacterMappingOverride>> {
    let mut matches = overrides
        .iter()
        .filter(|item| {
            item.source_server == resource.source_server
                && item.source_character == resource.source_character
                && match (&resource.source_account, &item.source_account) {
                    (Some(resource_account), Some(mapping_account)) => {
                        resource_account == mapping_account
                    }
                    (Some(_), None) => true,
                    (None, Some(_)) => false,
                    (None, None) => true,
                }
        })
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.pop()),
        _ => Err(AppError::Validation(format!(
            "multiple mapping overrides matched `{}/{}`",
            resource.source_server, resource.source_character
        ))),
    }
}

pub(super) fn find_character_mapping<'a>(
    mappings: &'a [CharacterMapping],
    source_account: &str,
    source_server: &str,
    source_character: &str,
) -> Option<&'a CharacterMapping> {
    mappings.iter().find(|mapping| {
        mapping.source_account.as_deref() == Some(source_account)
            && mapping.source_server == source_server
            && mapping.source_character == source_character
    })
}
