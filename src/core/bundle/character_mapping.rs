use super::character_mapping_match::resolve_mapping_override;
use super::*;

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

fn validate_character_mapping_inputs(
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

fn build_keep_original_character_mapping(
    resource: &CharacterResource,
    source_account: Option<String>,
) -> AppResult<CharacterMapping> {
    let target_account = source_account.clone().ok_or_else(|| {
        AppError::Validation(format!(
            "source account is required for keep_original character mapping on `{}/{}`",
            resource.source_server, resource.source_character
        ))
    })?;
    validate_plain_name("target account", &target_account)?;
    validate_plain_name("target server", &resource.source_server)?;
    validate_plain_name("target character", &resource.source_character)?;

    Ok(CharacterMapping {
        source_account,
        source_server: resource.source_server.clone(),
        source_character: resource.source_character.clone(),
        target_account,
        target_server: resource.source_server.clone(),
        target_character: resource.source_character.clone(),
    })
}

fn build_resolved_character_mapping(
    character_mode: CharacterMappingMode,
    resource: &CharacterResource,
    source_account: Option<String>,
    apply_mappings: &BundleApplyMappings,
    single_character_bundle: bool,
) -> AppResult<CharacterMapping> {
    let override_mapping = resolve_mapping_override(resource, &apply_mappings.characters)?;
    let target_account = override_mapping
        .and_then(|item| item.target_account.clone())
        .or_else(|| apply_mappings.target_account.clone());
    let target_server = override_mapping
        .map(|item| item.target_server.clone())
        .or_else(|| {
            if single_character_bundle {
                apply_mappings.target_server.clone()
            } else {
                None
            }
        });
    let target_character = override_mapping
        .map(|item| item.target_character.clone())
        .or_else(|| {
            if single_character_bundle {
                apply_mappings.target_character.clone()
            } else {
                None
            }
        });

    let mut missing_fields = Vec::new();
    if target_account.is_none() {
        missing_fields.push("target_account");
    }
    if target_server.is_none() {
        missing_fields.push("target_server");
    }
    if target_character.is_none() {
        missing_fields.push("target_character");
    }

    if !missing_fields.is_empty() {
        return Err(AppError::Validation(
            format_character_mapping_resolution_error(
                character_mode,
                resource,
                single_character_bundle,
                &missing_fields,
            ),
        ));
    }

    let target_account = target_account.expect("validated target account");
    let target_server = target_server.expect("validated target server");
    let target_character = target_character.expect("validated target character");

    validate_plain_name("target account", &target_account)?;
    validate_plain_name("target server", &target_server)?;
    validate_plain_name("target character", &target_character)?;

    Ok(CharacterMapping {
        source_account,
        source_server: resource.source_server.clone(),
        source_character: resource.source_character.clone(),
        target_account,
        target_server,
        target_character,
    })
}

fn format_character_mapping_resolution_error(
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
