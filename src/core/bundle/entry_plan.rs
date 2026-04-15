use std::collections::BTreeMap;

use super::*;

pub(super) fn plan_extractable_entries(
    entry_names: &[String],
    installation: &DetectedFlavorInstallation,
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
    selected_target_accounts: &[String],
) -> AppResult<Vec<PlannedEntry>> {
    let mut planned_entries = Vec::new();
    let common_account_targets = resolve_common_account_targets(
        manifest,
        character_mappings,
        apply_mappings,
        selected_target_accounts,
    )?;

    for archive_name in entry_names {
        let entries = map_bundle_entry_to_destination(
            archive_name,
            installation,
            manifest,
            character_mappings,
            &common_account_targets,
            apply_mappings.target_account.as_deref(),
            selected_target_accounts,
        )?;

        planned_entries.extend(entries);
    }

    Ok(planned_entries)
}

fn map_bundle_entry_to_destination(
    archive_name: &str,
    installation: &DetectedFlavorInstallation,
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    common_account_targets: &BTreeMap<String, String>,
    default_target_account: Option<&str>,
    selected_target_accounts: &[String],
) -> AppResult<Vec<PlannedEntry>> {
    if archive_name == MANIFEST_ENTRY {
        return Ok(Vec::new());
    }

    let segments = safe_zip_segments(archive_name)?;
    if segments.is_empty() {
        return Ok(Vec::new());
    }

    match segments.as_slice() {
        ["metadata", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(
                &installation
                    .addon_dir
                    .join(".hearthsync")
                    .join("bundles")
                    .join(safe_file_part(&manifest.package.id)),
                rest,
            ),
            rewrites: Vec::new(),
            group: ApplyGroup::Metadata,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        ["addons", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.addon_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::Addons,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        ["wtf", "common", "Config.wtf"] => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: installation.wtf_dir.join("Config.wtf"),
            rewrites: Vec::new(),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::GlobalConfig),
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        [
            "wtf",
            "common",
            "accounts",
            source_account,
            "SavedVariables",
            rest @ ..,
        ] if !rest.is_empty() => {
            let target_accounts = if !selected_target_accounts.is_empty() {
                selected_target_accounts.to_vec()
            } else {
                vec![
                    common_account_targets
                        .get(*source_account)
                        .cloned()
                        .or_else(|| default_target_account.map(|item| item.to_string()))
                        .unwrap_or_else(|| (*source_account).to_string()),
                ]
            };

            Ok(target_accounts
                .into_iter()
                .map(|target_account| PlannedEntry {
                    archive_name: archive_name.to_string(),
                    destination: installation
                        .wtf_dir
                        .join("Account")
                        .join(&target_account)
                        .join("SavedVariables")
                        .join(join_segments(Path::new(""), rest)),
                    rewrites: character_mappings
                        .iter()
                        .filter(|mapping| mapping.target_account == target_account)
                        .cloned()
                        .collect::<Vec<_>>(),
                    group: ApplyGroup::WtfCommon,
                    wtf_scope: Some(WtfScope::AccountSavedVariables),
                    target_account: Some(target_account),
                    target_server: None,
                    target_character: None,
                })
                .collect())
        }
        ["wtf", "common", "accounts", source_account, rest @ ..] if !rest.is_empty() => {
            let target_accounts = if !selected_target_accounts.is_empty() {
                selected_target_accounts.to_vec()
            } else {
                vec![
                    common_account_targets
                        .get(*source_account)
                        .cloned()
                        .or_else(|| default_target_account.map(|item| item.to_string()))
                        .unwrap_or_else(|| (*source_account).to_string()),
                ]
            };

            Ok(target_accounts
                .into_iter()
                .map(|target_account| PlannedEntry {
                    archive_name: archive_name.to_string(),
                    destination: installation
                        .wtf_dir
                        .join("Account")
                        .join(&target_account)
                        .join(join_segments(Path::new(""), rest)),
                    rewrites: character_mappings
                        .iter()
                        .filter(|mapping| mapping.target_account == target_account)
                        .cloned()
                        .collect::<Vec<_>>(),
                    group: ApplyGroup::WtfCommon,
                    wtf_scope: Some(classify_account_wtf_scope(rest)),
                    target_account: Some(target_account),
                    target_server: None,
                    target_character: None,
                })
                .collect())
        }
        [
            "wtf",
            "characters",
            source_account,
            server,
            character,
            rest @ ..,
        ] if !rest.is_empty() => {
            let mapping =
                find_character_mapping(character_mappings, source_account, server, character)
                    .cloned()
                    .unwrap_or_else(|| CharacterMapping {
                        source_account: Some((*source_account).to_string()),
                        source_server: (*server).to_string(),
                        source_character: (*character).to_string(),
                        target_account: (*source_account).to_string(),
                        target_server: (*server).to_string(),
                        target_character: (*character).to_string(),
                    });

            Ok(vec![PlannedEntry {
                archive_name: archive_name.to_string(),
                destination: installation
                    .wtf_dir
                    .join("Account")
                    .join(&mapping.target_account)
                    .join(&mapping.target_server)
                    .join(&mapping.target_character)
                    .join(join_segments(Path::new(""), rest)),
                rewrites: vec![mapping.clone()],
                group: ApplyGroup::WtfCharacters,
                wtf_scope: Some(classify_character_wtf_scope(rest)),
                target_account: Some(mapping.target_account),
                target_server: Some(mapping.target_server),
                target_character: Some(mapping.target_character),
            }])
        }
        ["fonts", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.fonts_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::Fonts,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        ["interface", rest @ ..] if !rest.is_empty() => Ok(vec![PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: join_segments(&installation.interface_dir, rest),
            rewrites: Vec::new(),
            group: ApplyGroup::InterfaceAssets,
            wtf_scope: None,
            target_account: None,
            target_server: None,
            target_character: None,
        }]),
        _ => Ok(Vec::new()),
    }
}

fn classify_account_wtf_scope(relative_segments: &[&str]) -> WtfScope {
    if relative_segments.is_empty() {
        return WtfScope::Unknown;
    }

    if is_saved_variables_segment(relative_segments[0]) {
        WtfScope::AccountSavedVariables
    } else if relative_segments
        .last()
        .is_some_and(|name| is_cache_like_wtf_file_name(name))
    {
        WtfScope::CacheLike
    } else {
        WtfScope::AccountRootFile
    }
}

fn classify_character_wtf_scope(relative_segments: &[&str]) -> WtfScope {
    if relative_segments.is_empty() {
        return WtfScope::Unknown;
    }

    if is_saved_variables_segment(relative_segments[0]) {
        WtfScope::CharacterSavedVariables
    } else if relative_segments
        .last()
        .is_some_and(|name| is_cache_like_wtf_file_name(name))
    {
        WtfScope::CacheLike
    } else {
        WtfScope::CharacterState
    }
}

fn is_saved_variables_segment(segment: &str) -> bool {
    segment.eq_ignore_ascii_case("SavedVariables")
}

fn is_cache_like_wtf_file_name(file_name: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    matches!(
        file_name.as_str(),
        "bindings-cache.wtf" | "chat-cache.txt" | "config-cache.wtf" | "macros-cache.txt"
    ) || file_name.ends_with("-cache.wtf")
        || file_name.ends_with("-cache.txt")
        || file_name.ends_with("-cache.old")
}
