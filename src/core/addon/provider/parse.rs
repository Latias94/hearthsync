use super::{AddonSourceRef, validate_addon_source_ref};
use crate::core::error::{AppError, AppResult};

const GITHUB_SOURCE_INPUT_MESSAGE: &str =
    "GitHub source must look like `github:owner/repo[@tag][#asset.zip]`";

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
        Some((left, right)) => {
            let asset_name = right.trim();
            if asset_name.is_empty() {
                return Err(AppError::Validation(
                    GITHUB_SOURCE_INPUT_MESSAGE.to_string(),
                ));
            }
            (left, Some(asset_name.to_string()))
        }
        None => (spec, None),
    };
    let (repo_spec, tag) = match repo_and_tag.rsplit_once('@') {
        Some((left, right)) if left.contains('/') => {
            let tag = right.trim();
            if tag.is_empty() {
                return Err(AppError::Validation(
                    GITHUB_SOURCE_INPUT_MESSAGE.to_string(),
                ));
            }
            (left, Some(tag.to_string()))
        }
        Some(_) => {
            return Err(AppError::Validation(
                GITHUB_SOURCE_INPUT_MESSAGE.to_string(),
            ));
        }
        _ => (repo_and_tag, None),
    };
    let Some((owner, repo)) = repo_spec.split_once('/') else {
        return Err(AppError::Validation(
            GITHUB_SOURCE_INPUT_MESSAGE.to_string(),
        ));
    };

    let owner = owner.trim();
    let repo = repo.trim();
    if owner.is_empty() || repo.is_empty() {
        return Err(AppError::Validation(
            GITHUB_SOURCE_INPUT_MESSAGE.to_string(),
        ));
    }

    let source_ref = AddonSourceRef::GitHubRelease {
        owner: owner.to_string(),
        repo: repo.to_string(),
        tag,
        asset_name,
    };
    validate_addon_source_ref(&source_ref, "GitHub source input")
        .map_err(|_| AppError::Validation(GITHUB_SOURCE_INPUT_MESSAGE.to_string()))?;

    Ok(Some(source_ref))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_curseforge_source_with_explicit_file() {
        let source = parse_curseforge_source("curseforge:12345@67890")
            .expect("parse")
            .expect("source ref");

        assert_eq!(
            source,
            AddonSourceRef::CurseForgeMod {
                mod_id: 12345,
                file_id: Some(67890),
            }
        );
    }

    #[test]
    fn parse_curseforge_source_without_file() {
        let source = parse_curseforge_source("curseforge:12345")
            .expect("parse")
            .expect("source ref");

        assert_eq!(
            source,
            AddonSourceRef::CurseForgeMod {
                mod_id: 12345,
                file_id: None,
            }
        );
    }

    #[test]
    fn parse_curseforge_source_rejects_zero_ids() {
        for source in ["curseforge:0", "curseforge:12345@0"] {
            let error = parse_curseforge_source(source).expect_err("zero ids should fail");

            assert!(matches!(error, AppError::Validation(_)));
            assert!(
                error
                    .to_string()
                    .contains("CurseForge source must look like")
            );
        }
    }

    #[test]
    fn parse_github_source_with_tag_and_asset() {
        let source = parse_github_source("github:owner/repo@v1.2.3#addon.zip")
            .expect("parse")
            .expect("source ref");

        assert_eq!(
            source,
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: Some("v1.2.3".to_string()),
                asset_name: Some("addon.zip".to_string()),
            }
        );
    }

    #[test]
    fn parse_github_source_accepts_tag_path_segments() {
        let source = parse_github_source("github:owner/repo@retail/2026.05#addon.zip")
            .expect("parse")
            .expect("source ref");

        assert_eq!(
            source,
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: Some("retail/2026.05".to_string()),
                asset_name: Some("addon.zip".to_string()),
            }
        );
    }

    #[test]
    fn parse_github_source_without_tag() {
        let source = parse_github_source("github:owner/repo")
            .expect("parse")
            .expect("source ref");

        assert_eq!(
            source,
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: None,
            }
        );
    }

    #[test]
    fn parse_github_source_rejects_empty_tag_or_asset() {
        for source in [
            "github:owner/repo@",
            "github:owner/repo#",
            "github:owner/repo@#addon.zip",
        ] {
            let error = parse_github_source(source).expect_err("invalid github source");

            assert!(matches!(error, AppError::Validation(_)));
            assert!(error.to_string().contains("GitHub source must look like"));
        }
    }

    #[test]
    fn parse_github_source_rejects_unsafe_repository_or_asset_segments() {
        for source in [
            "github:bad/owner/repo",
            "github:owner/repo name",
            "github:owner/repo#bad/name.zip",
        ] {
            let error = parse_github_source(source).expect_err("invalid github source");

            assert!(matches!(error, AppError::Validation(_)));
            assert!(error.to_string().contains("GitHub source must look like"));
        }
    }
}
