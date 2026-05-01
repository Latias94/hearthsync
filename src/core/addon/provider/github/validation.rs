use std::collections::BTreeSet;

use super::{GitHubRelease, GitHubReleaseAsset, is_zip_asset_name};
use crate::core::archive_path::validate_portable_path_segment;
use crate::core::boundary_validation::{
    is_rfc3339_timestamp_shape, validate_hex_digest, validate_http_url,
};
use crate::core::error::{AppError, AppResult};

pub(super) fn validate_github_releases(releases: &[GitHubRelease]) -> AppResult<()> {
    for release in releases {
        validate_github_release(release)?;
    }

    Ok(())
}

pub(super) fn validate_github_release(release: &GitHubRelease) -> AppResult<()> {
    if release.tag_name.trim().is_empty() {
        return Err(AppError::Validation(
            "GitHub release tag name must not be empty".to_string(),
        ));
    }
    if release.tag_name.trim() != release.tag_name {
        return Err(AppError::Validation(
            "GitHub release tag name must not have surrounding whitespace".to_string(),
        ));
    }

    let mut asset_names = BTreeSet::new();
    for asset in &release.assets {
        validate_github_release_asset(asset)?;
        let normalized_name = asset.name.to_ascii_lowercase();
        if !asset_names.insert(normalized_name) {
            return Err(AppError::Validation(format!(
                "GitHub release asset `{}` is duplicated under case-insensitive comparison",
                asset.name
            )));
        }
    }

    Ok(())
}

pub(super) fn validate_github_download_asset(asset: &GitHubReleaseAsset) -> AppResult<()> {
    validate_github_release_asset(asset)?;
    if !is_zip_asset_name(&asset.name) {
        return Err(AppError::Validation(format!(
            "GitHub release asset `{}` is not a `.zip` archive",
            asset.name
        )));
    }

    Ok(())
}

fn validate_github_release_asset(asset: &GitHubReleaseAsset) -> AppResult<()> {
    validate_portable_path_segment(&asset.name, "GitHub release asset")?;
    validate_http_url(
        &asset.browser_download_url,
        &format!("GitHub release asset `{}` download URL", asset.name),
    )?;
    if asset.size.is_some_and(|size| size == 0) {
        return Err(AppError::Validation(format!(
            "GitHub release asset `{}` size must be greater than zero",
            asset.name
        )));
    }
    if let Some(digest) = asset.digest.as_deref() {
        validate_github_asset_digest(asset, digest)?;
    }
    if let Some(updated_at) = asset.updated_at.as_deref()
        && !is_rfc3339_timestamp_shape(updated_at)
    {
        return Err(AppError::Validation(format!(
            "GitHub release asset `{}` updated_at must be an RFC 3339 timestamp",
            asset.name
        )));
    }

    Ok(())
}

fn validate_github_asset_digest(asset: &GitHubReleaseAsset, digest: &str) -> AppResult<()> {
    if digest.trim().is_empty() {
        return Err(AppError::Validation(format!(
            "GitHub release asset `{}` digest must not be empty",
            asset.name
        )));
    }
    if digest.trim() != digest {
        return Err(AppError::Validation(format!(
            "GitHub release asset `{}` digest must not have surrounding whitespace",
            asset.name
        )));
    }
    let Some(value) = digest.strip_prefix("sha256:") else {
        return Err(AppError::Validation(format!(
            "GitHub release asset `{}` digest must use the `sha256:` prefix",
            asset.name
        )));
    };

    validate_hex_digest(
        value,
        &format!("GitHub release asset `{}` digest", asset.name),
        64,
        "SHA-256",
    )
}
