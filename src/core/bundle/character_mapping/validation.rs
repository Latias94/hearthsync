use super::*;

pub(super) fn validate_character_mapping_inputs(
    character_mode: CharacterMappingMode,
    apply_mappings: &BundleApplyMappings,
    single_character_bundle: bool,
) -> AppResult<()> {
    if matches!(
        character_mode,
        CharacterMappingMode::Explicit | CharacterMappingMode::Prompt
    ) && !single_character_bundle
        && (apply_mappings.target_server.is_some() || apply_mappings.target_character.is_some())
    {
        return Err(AppError::Validation(
            "global target_server/target_character overrides are only supported when the bundle contains exactly one character; use `--mapping-file` for multi-character explicit mappings.".to_string(),
        ));
    }

    Ok(())
}

pub(super) fn format_character_mapping_resolution_error(
    character_mode: CharacterMappingMode,
    resource: &CharacterResource,
    single_character_bundle: bool,
    missing_fields: &[&str],
) -> String {
    let mode_message = match character_mode {
        CharacterMappingMode::KeepOriginal => "keep_original should not require target identity",
        CharacterMappingMode::Explicit => {
            "explicit character mode requires a fully resolved target identity"
        }
        CharacterMappingMode::Prompt => {
            "prompt character mode requires caller-provided target identity because the current CLI does not prompt automatically"
        }
    };
    let resolution = if single_character_bundle {
        "Provide `--target-account`, `--target-server`, and `--target-character`, or use `--mapping-file`."
    } else {
        "Provide per-character mappings with `--mapping-file`."
    };
    let hint = resource
        .target_hint
        .as_deref()
        .map(|hint| format!(" Hint: {hint}."))
        .unwrap_or_default();

    format!(
        "{mode_message} for `{}/{}` (missing: {}). {resolution}{hint}",
        resource.source_server,
        resource.source_character,
        missing_fields.join(", "),
    )
}
