use super::AddonSourceRef;
use crate::core::error::{AppError, AppResult};

pub(super) fn parse_curseforge_source(source: &str) -> AppResult<Option<AddonSourceRef>> {
    let Some(spec) = source.strip_prefix("curseforge:") else {
        return Ok(None);
    };

    let (mod_id_text, file_id_text) = match spec.split_once('@') {
        Some((left, right)) => (left, Some(right)),
        None => (spec, None),
    };
    let mod_id = parse_positive_u32(
        mod_id_text.trim(),
        "CurseForge source must look like `curseforge:<mod-id>[@file-id]`",
    )?;
    let file_id = match file_id_text {
        Some(value) => Some(parse_positive_u32(
            value.trim(),
            "CurseForge source must look like `curseforge:<mod-id>[@file-id]`",
        )?),
        None => None,
    };

    Ok(Some(AddonSourceRef::CurseForgeMod { mod_id, file_id }))
}

pub(super) fn parse_github_source(source: &str) -> AppResult<Option<AddonSourceRef>> {
    let Some(spec) = source.strip_prefix("github:") else {
        return Ok(None);
    };

    let (repo_and_tag, asset_name) = match spec.split_once('#') {
        Some((left, right)) => (left, Some(right.to_string())),
        None => (spec, None),
    };
    let (repo_spec, tag) = match repo_and_tag.rsplit_once('@') {
        Some((left, right)) if left.contains('/') && !right.trim().is_empty() => {
            (left, Some(right.to_string()))
        }
        _ => (repo_and_tag, None),
    };
    let Some((owner, repo)) = repo_spec.split_once('/') else {
        return Err(AppError::Validation(
            "GitHub source must look like `github:owner/repo[@tag][#asset.zip]`".to_string(),
        ));
    };

    let owner = owner.trim();
    let repo = repo.trim();
    if owner.is_empty() || repo.is_empty() {
        return Err(AppError::Validation(
            "GitHub source must look like `github:owner/repo[@tag][#asset.zip]`".to_string(),
        ));
    }

    Ok(Some(AddonSourceRef::GitHubRelease {
        owner: owner.to_string(),
        repo: repo.to_string(),
        tag,
        asset_name,
    }))
}

fn parse_positive_u32(value: &str, message: &str) -> AppResult<u32> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| AppError::Validation(message.to_string()))?;
    if parsed == 0 {
        return Err(AppError::Validation(message.to_string()));
    }

    Ok(parsed)
}
