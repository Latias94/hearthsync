use std::collections::BTreeSet;

use super::super::model::{CurseForgeGame, CurseForgeGameVersionType, CurseForgeSearchMod};
use crate::core::boundary_validation::validate_http_url;
use crate::core::error::{AppError, AppResult};

pub(super) fn validate_curseforge_games(games: &[CurseForgeGame]) -> AppResult<()> {
    let mut game_ids = BTreeSet::new();
    for game in games {
        if game.id == 0 {
            return Err(AppError::Validation(
                "CurseForge game id must be greater than zero".to_string(),
            ));
        }
        validate_required_provider_text(game.id, "CurseForge game name", &game.name)?;
        validate_required_provider_text(game.id, "CurseForge game slug", &game.slug)?;
        if !game_ids.insert(game.id) {
            return Err(AppError::Validation(format!(
                "CurseForge games response declares duplicate game id `{}`",
                game.id
            )));
        }
    }

    Ok(())
}

pub(super) fn validate_curseforge_game_version_types(
    version_types: &[CurseForgeGameVersionType],
) -> AppResult<()> {
    let mut version_type_ids = BTreeSet::new();
    for version_type in version_types {
        if version_type.id == 0 {
            return Err(AppError::Validation(
                "CurseForge game version type id must be greater than zero".to_string(),
            ));
        }
        validate_required_provider_text(
            version_type.id,
            "CurseForge game version type name",
            &version_type.name,
        )?;
        validate_required_provider_text(
            version_type.id,
            "CurseForge game version type slug",
            &version_type.slug,
        )?;
        if !version_type_ids.insert(version_type.id) {
            return Err(AppError::Validation(format!(
                "CurseForge game version types response declares duplicate version type id `{}`",
                version_type.id
            )));
        }
    }

    Ok(())
}

pub(super) fn validate_curseforge_search_mods(mods: &[CurseForgeSearchMod]) -> AppResult<()> {
    for mod_item in mods {
        validate_curseforge_search_mod(mod_item)?;
    }

    Ok(())
}

fn validate_curseforge_search_mod(mod_item: &CurseForgeSearchMod) -> AppResult<()> {
    if mod_item.id == 0 {
        return Err(AppError::Validation(
            "CurseForge search result mod id must be greater than zero".to_string(),
        ));
    }
    validate_required_provider_text(mod_item.id, "CurseForge search result name", &mod_item.name)?;
    if let Some(website_url) = &mod_item.links.website_url {
        validate_http_url(
            website_url,
            &format!("CurseForge search result `{}` website URL", mod_item.id),
        )?;
    }
    for file_index in &mod_item.latest_files_indexes {
        if file_index.file_id == 0 {
            return Err(AppError::Validation(format!(
                "CurseForge search result `{}` latest file index file id must be greater than zero",
                mod_item.id
            )));
        }
        if file_index.game_version_type_id == 0 {
            return Err(AppError::Validation(format!(
                "CurseForge search result `{}` latest file index game version type id must be greater than zero",
                mod_item.id
            )));
        }
    }

    Ok(())
}

fn validate_required_provider_text(id: u32, field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "{field} `{id}` must not be empty"
        )));
    }
    if value.trim() != value {
        return Err(AppError::Validation(format!(
            "{field} `{id}` must not have surrounding whitespace"
        )));
    }

    Ok(())
}
