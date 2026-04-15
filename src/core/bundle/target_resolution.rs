use std::collections::BTreeMap;

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

pub(super) fn resolve_selected_target_accounts(
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

    if discovered_accounts.len() == 1 {
        return Ok(vec![discovered_accounts[0].account_name.clone()]);
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

    if discovered_accounts.is_empty() {
        let mut source_accounts = manifest
            .resources
            .wtf_characters
            .iter()
            .filter_map(|character| character.source_account.clone())
            .collect::<Vec<_>>();
        source_accounts.sort();
        source_accounts.dedup();
        if source_accounts.len() == 1 {
            return Ok(source_accounts);
        }
        return Ok(Vec::new());
    }

    Err(AppError::Validation(
        "common WTF resources require explicit target account selection. Use `--select-account`, `--all-accounts`, or `--target-account`.".to_string(),
    ))
}

fn resolve_mapping_override<'a>(
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

pub(super) fn resolve_common_account_targets(
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
    selected_target_accounts: &[String],
) -> AppResult<BTreeMap<String, String>> {
    let mut targets = BTreeMap::new();

    if !manifest.resources.wtf_common {
        return Ok(targets);
    }

    if !selected_target_accounts.is_empty() {
        return Ok(targets);
    }

    for mapping in character_mappings {
        let Some(source_account) = &mapping.source_account else {
            continue;
        };

        match targets.get(source_account) {
            Some(existing) if existing != &mapping.target_account => {
                return Err(AppError::Validation(format!(
                    "source account `{source_account}` maps to multiple target accounts (`{existing}` and `{}`), which is unsafe for common WTF resources",
                    mapping.target_account
                )));
            }
            Some(_) => {}
            None => {
                targets.insert(source_account.clone(), mapping.target_account.clone());
            }
        }
    }

    if let Some(default_target_account) = &apply_mappings.target_account {
        validate_plain_name("target account", default_target_account)?;
        for override_mapping in &apply_mappings.characters {
            if let Some(source_account) = &override_mapping.source_account {
                let target_account = override_mapping
                    .target_account
                    .clone()
                    .unwrap_or_else(|| default_target_account.clone());
                match targets.get(source_account) {
                    Some(existing) if existing != &target_account => {
                        return Err(AppError::Validation(format!(
                            "source account `{source_account}` maps to multiple target accounts (`{existing}` and `{target_account}`)"
                        )));
                    }
                    Some(_) => {}
                    None => {
                        targets.insert(source_account.clone(), target_account);
                    }
                }
            }
        }
    }

    Ok(targets)
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

pub(super) fn validate_target_compatibility(
    manifest: &BundleManifest,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    if !manifest.source.supported_targets.is_empty()
        && !manifest
            .source
            .supported_targets
            .contains(&installation.flavor)
    {
        return Err(AppError::Validation(format!(
            "bundle does not support target flavor `{}`",
            installation.flavor.as_str()
        )));
    }

    if let Some(source_platform) = manifest.source.platform {
        if source_platform != installation.platform && !manifest.mapping.allow_cross_platform {
            return Err(AppError::Validation(
                "bundle was exported on another platform, but allow_cross_platform is false"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
