use std::collections::BTreeMap;

use super::super::shared::validate_plain_name;
use super::super::types::BundleApplyMappings;
use crate::core::error::{AppError, AppResult};
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::BundleManifest;

pub(in crate::core::bundle) fn resolve_common_account_targets(
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
