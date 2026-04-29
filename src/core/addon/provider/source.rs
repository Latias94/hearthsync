use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddonSourceRef {
    LocalArchive {
        path: PathBuf,
    },
    HttpArchive {
        url: String,
    },
    #[serde(rename = "curseforge_mod", alias = "curse_forge_mod")]
    CurseForgeMod {
        mod_id: u32,
        file_id: Option<u32>,
    },
    #[serde(rename = "github_release", alias = "git_hub_release")]
    GitHubRelease {
        owner: String,
        repo: String,
        tag: Option<String>,
        asset_name: Option<String>,
    },
}

impl AddonSourceRef {
    pub fn display_name(&self) -> String {
        match self {
            Self::LocalArchive { path } => path.display().to_string(),
            Self::HttpArchive { url } => url.clone(),
            Self::CurseForgeMod { mod_id, file_id } => {
                let mut text = format!("curseforge:{mod_id}");
                if let Some(file_id) = file_id {
                    text.push('@');
                    text.push_str(&file_id.to_string());
                }
                text
            }
            Self::GitHubRelease {
                owner,
                repo,
                tag,
                asset_name,
            } => {
                let mut text = format!("github:{owner}/{repo}");
                if let Some(tag) = tag {
                    text.push('@');
                    text.push_str(tag);
                }
                if let Some(asset_name) = asset_name {
                    text.push('#');
                    text.push_str(asset_name);
                }
                text
            }
        }
    }
}

pub(crate) fn addon_source_input_is_local_archive(source: &str) -> bool {
    !(source.starts_with("https://")
        || source.starts_with("http://")
        || source.starts_with("curseforge:")
        || source.starts_with("github:"))
}

pub(crate) fn validate_absolute_local_archive_source_path(path: &Path) -> AppResult<()> {
    if path.is_absolute() {
        return Ok(());
    }

    Err(AppError::Validation(format!(
        "local archive source path must be absolute before it reaches the addon core: {}",
        path.display()
    )))
}

pub(crate) fn validate_addon_source_ref(
    source: &AddonSourceRef,
    source_context: &str,
) -> AppResult<()> {
    match source {
        AddonSourceRef::LocalArchive { path } => {
            if path.as_os_str().is_empty() {
                return Err(invalid_source_ref(
                    source_context,
                    "local archive source path must not be empty",
                ));
            }
        }
        AddonSourceRef::HttpArchive { url } => {
            if url.trim().is_empty() {
                return Err(invalid_source_ref(
                    source_context,
                    "HTTP archive source URL must not be empty",
                ));
            }
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(invalid_source_ref(
                    source_context,
                    "HTTP archive source URL must start with `http://` or `https://`",
                ));
            }
        }
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => {
            if *mod_id == 0 {
                return Err(invalid_source_ref(
                    source_context,
                    "CurseForge mod id must be greater than zero",
                ));
            }
            if matches!(file_id, Some(0)) {
                return Err(invalid_source_ref(
                    source_context,
                    "CurseForge file id must be greater than zero",
                ));
            }
        }
        AddonSourceRef::GitHubRelease {
            owner,
            repo,
            tag,
            asset_name,
        } => {
            validate_required_source_text(source_context, "GitHub owner", owner)?;
            validate_required_source_text(source_context, "GitHub repo", repo)?;
            validate_optional_source_text(source_context, "GitHub tag", tag.as_deref())?;
            validate_optional_source_text(
                source_context,
                "GitHub asset name",
                asset_name.as_deref(),
            )?;
        }
    }

    Ok(())
}

fn validate_required_source_text(source_context: &str, field: &str, value: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(invalid_source_ref(
            source_context,
            &format!("{field} must not be empty"),
        ));
    }

    Ok(())
}

fn validate_optional_source_text(
    source_context: &str,
    field: &str,
    value: Option<&str>,
) -> AppResult<()> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        return Err(invalid_source_ref(
            source_context,
            &format!("{field} must not be empty when present"),
        ));
    }

    Ok(())
}

fn invalid_source_ref(source_context: &str, message: &str) -> AppError {
    AppError::Validation(format!("invalid {source_context}: {message}"))
}

pub(crate) fn canonicalize_local_archive_path(path: &Path) -> AppResult<PathBuf> {
    let resolved =
        fs::canonicalize(path).map_err(|_| AppError::NotFound(path.display().to_string()))?;
    if !resolved.is_file() {
        return Err(AppError::Validation(format!(
            "addon source must be a file archive: {}",
            resolved.display()
        )));
    }

    Ok(normalize_canonical_archive_path(resolved))
}

pub(super) fn source_cache_namespace(source: &AddonSourceRef) -> &'static str {
    match source {
        AddonSourceRef::LocalArchive { .. } => "local",
        AddonSourceRef::HttpArchive { .. } => "http",
        AddonSourceRef::CurseForgeMod { .. } => "curseforge",
        AddonSourceRef::GitHubRelease { .. } => "github",
    }
}

pub(super) fn source_kind_label(source: &AddonSourceRef) -> &'static str {
    match source {
        AddonSourceRef::LocalArchive { .. } => "local_archive",
        AddonSourceRef::HttpArchive { .. } => "http_archive",
        AddonSourceRef::CurseForgeMod { .. } => "curseforge_mod",
        AddonSourceRef::GitHubRelease { .. } => "github_release",
    }
}

pub(super) fn short_hash(value: &str) -> String {
    use std::fmt::Write as _;

    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::with_capacity(16);
    for byte in digest.iter().take(8) {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(windows)]
fn normalize_canonical_archive_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(stripped) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{}", stripped));
    }
    if let Some(stripped) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(stripped);
    }
    path
}

#[cfg(not(windows))]
fn normalize_canonical_archive_path(path: PathBuf) -> PathBuf {
    path
}
