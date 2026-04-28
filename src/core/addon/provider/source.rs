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
