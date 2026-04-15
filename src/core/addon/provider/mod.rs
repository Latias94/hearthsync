mod curseforge;
mod github;
mod parse;
#[cfg(test)]
mod tests;

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use self::curseforge::{resolve_curseforge_file, search_curseforge_mods};
use self::github::{fetch_github_release, select_github_release_asset};
use self::parse::{parse_curseforge_source, parse_github_source};
use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

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

pub struct MaterializedAddonSource {
    pub source_ref: AddonSourceRef,
    pub archive_path: PathBuf,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AddonProviderContext {
    pub target_flavor: Option<WowFlavor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSearchResult {
    pub provider: &'static str,
    pub name: String,
    pub summary: Option<String>,
    pub source: AddonSourceRef,
    pub install_hint: String,
    pub website_url: Option<String>,
    pub provider_project_id: Option<u32>,
    pub provider_file_id: Option<u32>,
    pub download_count: u64,
}

pub fn materialize_source_input(
    source: &str,
    stage_root: &Path,
    context: AddonProviderContext,
) -> AppResult<MaterializedAddonSource> {
    if let Some(source_ref) = parse_curseforge_source(source)? {
        return materialize_source_ref(&source_ref, stage_root, context);
    }

    if let Some(source_ref) = parse_github_source(source)? {
        return materialize_source_ref(&source_ref, stage_root, context);
    }

    if source.starts_with("https://") || source.starts_with("http://") {
        let source_ref = AddonSourceRef::HttpArchive {
            url: source.to_string(),
        };
        return materialize_source_ref(&source_ref, stage_root, context);
    }

    let path = fs::canonicalize(source).map_err(|_| AppError::NotFound(source.to_string()))?;
    if !path.is_file() {
        return Err(AppError::Validation(format!(
            "addon source must be a file archive: {}",
            path.display()
        )));
    }

    Ok(MaterializedAddonSource {
        source_ref: AddonSourceRef::LocalArchive { path: path.clone() },
        archive_path: path,
    })
}

pub fn materialize_source_ref(
    source: &AddonSourceRef,
    stage_root: &Path,
    context: AddonProviderContext,
) -> AppResult<MaterializedAddonSource> {
    match source {
        AddonSourceRef::LocalArchive { path } => Ok(MaterializedAddonSource {
            source_ref: source.clone(),
            archive_path: path.clone(),
        }),
        AddonSourceRef::HttpArchive { url } => {
            let file_name = guess_archive_name_from_url(url).unwrap_or("downloaded-addon.zip");
            let archive_path = stage_root.join(file_name);
            download_to_path(url, &archive_path)?;
            Ok(MaterializedAddonSource {
                source_ref: source.clone(),
                archive_path,
            })
        }
        AddonSourceRef::CurseForgeMod { mod_id, file_id } => {
            let file = resolve_curseforge_file(*mod_id, *file_id, context.target_flavor)?;
            let download_url = file.download_url.clone().ok_or_else(|| {
                AppError::Validation(format!(
                    "CurseForge file `{}` does not provide a download URL",
                    file.id
                ))
            })?;
            let archive_path = stage_root.join(&file.file_name);
            download_to_path(&download_url, &archive_path)?;
            Ok(MaterializedAddonSource {
                source_ref: source.clone(),
                archive_path,
            })
        }
        AddonSourceRef::GitHubRelease {
            owner,
            repo,
            tag,
            asset_name,
        } => {
            let release = fetch_github_release(owner, repo, tag.as_deref())?;
            let asset = select_github_release_asset(&release, asset_name.as_deref())?;
            let archive_path = stage_root.join(&asset.name);
            download_to_path(&asset.browser_download_url, &archive_path)?;
            Ok(MaterializedAddonSource {
                source_ref: source.clone(),
                archive_path,
            })
        }
    }
}

pub fn search_addons(
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<Vec<AddonSearchResult>> {
    search_curseforge_mods(query, flavor, limit)
}

fn download_to_path(url: &str, destination: &Path) -> AppResult<()> {
    let client = Client::builder().build()?;
    let mut response = client.get(url).send()?.error_for_status()?;
    let mut file = File::create(destination)?;
    response.copy_to(&mut file)?;
    Ok(())
}

fn guess_archive_name_from_url(url: &str) -> Option<&str> {
    let file_name = Path::new(url).file_name()?.to_str()?;
    if file_name.is_empty() {
        None
    } else {
        Some(file_name)
    }
}
