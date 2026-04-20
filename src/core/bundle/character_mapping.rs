mod resolution;
mod validation;

use super::types::apply::BundleApplyMappings;
use crate::core::error::AppResult;
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::{BundleManifest, CharacterMappingMode};
use resolution::{build_keep_original_character_mapping, build_resolved_character_mapping};
use validation::validate_character_mapping_inputs;

pub(super) fn build_character_mappings(
    manifest: &BundleManifest,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<Vec<CharacterMapping>> {
    let single_character_bundle = manifest.resources.wtf_characters.len() == 1;
    validate_character_mapping_inputs(
        manifest.mapping.character_mode,
        apply_mappings,
        single_character_bundle,
    )?;
    let mut mappings = Vec::new();

    for resource in &manifest.resources.wtf_characters {
        let source_account = resource.source_account.clone();
        let mapping = match manifest.mapping.character_mode {
            CharacterMappingMode::KeepOriginal => {
                build_keep_original_character_mapping(resource, source_account)?
            }
            CharacterMappingMode::Explicit | CharacterMappingMode::Prompt => {
                build_resolved_character_mapping(
                    manifest.mapping.character_mode,
                    resource,
                    source_account,
                    apply_mappings,
                    single_character_bundle,
                )?
            }
        };

        mappings.push(mapping);
    }

    Ok(mappings)
}
