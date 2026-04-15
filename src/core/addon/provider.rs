use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

use reqwest::StatusCode;
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};

use crate::core::error::{AppError, AppResult};
use crate::core::install::WowFlavor;

const CURSEFORGE_API_BASE: &str = "https://api.curseforge.com/v1";
const CURSEFORGE_ACCEPT: &str = "application/json";
const CURSEFORGE_API_KEY_ENV: &str = "HEARTHSYNC_CURSEFORGE_API_KEY";
const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_API_VERSION: &str = "2022-11-28";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";
const USER_AGENT_VALUE: &str = "hearthsync/0.1.0";

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

fn parse_curseforge_source(source: &str) -> AppResult<Option<AddonSourceRef>> {
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

fn parse_github_source(source: &str) -> AppResult<Option<AddonSourceRef>> {
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
    value
        .parse::<u32>()
        .map_err(|_| AppError::Validation(message.to_string()))
}

fn parse_positive_usize(value: usize) -> usize {
    value.max(1).min(50)
}

fn resolve_curseforge_wow_context(flavor: WowFlavor) -> AppResult<CurseForgeWowContext> {
    let game_id = find_curseforge_wow_game_id()?;
    let version_types = fetch_curseforge_game_version_types(game_id)?;
    let version_type = select_curseforge_version_type(&version_types, flavor)?;

    Ok(CurseForgeWowContext {
        game_id,
        version_type_id: version_type.id,
    })
}

fn find_curseforge_wow_game_id() -> AppResult<u32> {
    let client = curseforge_client()?;
    let mut index = 0usize;

    loop {
        let url = format!("{CURSEFORGE_API_BASE}/games?index={index}&pageSize=50");
        let response = send_curseforge_request(client.get(url))?;
        let payload = serde_json::from_str::<CurseForgePaginatedResponse<Vec<CurseForgeGame>>>(
            &response.text()?,
        )?;
        let games = payload.data;
        if let Some(game) = games.iter().find(|game| is_world_of_warcraft_game(game)) {
            return Ok(game.id);
        }
        if games.len() < 50 {
            break;
        }
        index += 50;
    }

    Err(AppError::NotFound(
        "CurseForge game `World of Warcraft` was not found for the current API key".to_string(),
    ))
}

fn fetch_curseforge_game_version_types(game_id: u32) -> AppResult<Vec<CurseForgeGameVersionType>> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/games/{game_id}/version-types");
    let response = send_curseforge_request(client.get(url))?;
    let payload = serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeGameVersionType>>>(
        &response.text()?,
    )?;
    Ok(payload.data)
}

fn select_curseforge_version_type(
    version_types: &[CurseForgeGameVersionType],
    flavor: WowFlavor,
) -> AppResult<CurseForgeGameVersionType> {
    let candidates = curseforge_version_type_candidates(flavor);
    version_types
        .iter()
        .find(|version_type| {
            let slug = version_type.slug.to_ascii_lowercase();
            let name = version_type.name.to_ascii_lowercase();
            candidates.iter().any(|candidate| {
                slug == *candidate
                    || slug.contains(candidate)
                    || name == *candidate
                    || name.contains(candidate)
            })
        })
        .cloned()
        .ok_or_else(|| {
            AppError::Validation(format!(
                "CurseForge version type for flavor `{}` was not found. Available version types: {}",
                flavor.as_str(),
                version_types
                    .iter()
                    .map(|item| format!("{}({})", item.slug, item.id))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })
}

fn curseforge_version_type_candidates(flavor: WowFlavor) -> &'static [&'static str] {
    match flavor {
        WowFlavor::Retail => &["wow_retail", "retail"],
        WowFlavor::Classic => &["wow_classic", "classic"],
        WowFlavor::ClassicEra => &["wow_classic_era", "classic_era", "classic era"],
        WowFlavor::Ptr => &["wow_ptr", "ptr"],
        WowFlavor::Beta => &["wow_beta", "beta"],
        WowFlavor::Xptr => &["wow_xptr", "xptr"],
    }
}

fn is_world_of_warcraft_game(game: &CurseForgeGame) -> bool {
    let name = game.name.to_ascii_lowercase();
    let slug = game.slug.to_ascii_lowercase();
    name == "world of warcraft"
        || slug == "world-of-warcraft"
        || slug == "world_of_warcraft"
        || (name.contains("world") && name.contains("warcraft"))
}

fn curseforge_client() -> AppResult<Client> {
    let api_key = env::var(CURSEFORGE_API_KEY_ENV).map_err(|_| {
        AppError::Validation(format!(
            "CurseForge provider requires environment variable `{CURSEFORGE_API_KEY_ENV}`"
        ))
    })?;
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static(CURSEFORGE_ACCEPT));
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(&api_key).map_err(|error| AppError::Validation(error.to_string()))?,
    );

    Ok(Client::builder().default_headers(headers).build()?)
}

fn send_curseforge_request(
    request: reqwest::blocking::RequestBuilder,
) -> AppResult<reqwest::blocking::Response> {
    let response = request.send()?;
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let message = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => format!(
            "CurseForge request was rejected with {}. Check `{CURSEFORGE_API_KEY_ENV}` and ensure the API key is valid for the official CurseForge REST API.",
            status
        ),
        _ => format!("CurseForge request failed with HTTP status {status}"),
    };

    Err(AppError::Validation(message))
}

fn github_client() -> AppResult<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static(GITHUB_ACCEPT));
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_VALUE));
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static(GITHUB_API_VERSION),
    );

    Ok(Client::builder().default_headers(headers).build()?)
}

fn fetch_github_release(owner: &str, repo: &str, tag: Option<&str>) -> AppResult<GitHubRelease> {
    let client = github_client()?;
    let url = match tag {
        Some(tag) => format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases/tags/{tag}"),
        None => format!("{GITHUB_API_BASE}/repos/{owner}/{repo}/releases/latest"),
    };
    let response = client.get(url).send()?.error_for_status()?;
    Ok(serde_json::from_str(&response.text()?)?)
}

fn resolve_curseforge_file(
    mod_id: u32,
    file_id: Option<u32>,
    target_flavor: Option<WowFlavor>,
) -> AppResult<CurseForgeFile> {
    let wow_context = target_flavor
        .map(resolve_curseforge_wow_context)
        .transpose()?;
    if let Some(file_id) = file_id {
        let file = fetch_curseforge_file(mod_id, file_id)?;
        if let Some(wow_context) = &wow_context {
            ensure_curseforge_file_matches_version_type(&file, wow_context.version_type_id)?;
        }
        return validate_curseforge_file(file);
    }

    let files = fetch_curseforge_mod_files(mod_id)?;
    select_latest_curseforge_file(files, wow_context.as_ref().map(|item| item.version_type_id))
}

fn fetch_curseforge_mod_files(mod_id: u32) -> AppResult<Vec<CurseForgeFile>> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files");
    let response = send_curseforge_request(client.get(url))?;
    let payload =
        serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeFile>>>(&response.text()?)?;
    Ok(payload.data)
}

fn fetch_curseforge_file(mod_id: u32, file_id: u32) -> AppResult<CurseForgeFile> {
    let client = curseforge_client()?;
    let url = format!("{CURSEFORGE_API_BASE}/mods/{mod_id}/files/{file_id}");
    let response = send_curseforge_request(client.get(url))?;
    let payload = serde_json::from_str::<CurseForgeApiResponse<CurseForgeFile>>(&response.text()?)?;
    Ok(payload.data)
}

fn search_curseforge_mods(
    query: &str,
    flavor: WowFlavor,
    limit: usize,
) -> AppResult<Vec<AddonSearchResult>> {
    let wow_context = resolve_curseforge_wow_context(flavor)?;
    let client = curseforge_client()?;
    let page_size = parse_positive_usize(limit);
    let request = client
        .get(format!("{CURSEFORGE_API_BASE}/mods/search"))
        .query(&[
            ("gameId", wow_context.game_id.to_string()),
            ("gameVersionTypeId", wow_context.version_type_id.to_string()),
            ("searchFilter", query.to_string()),
            ("pageSize", page_size.to_string()),
        ]);
    let response = send_curseforge_request(request)?;
    let payload =
        serde_json::from_str::<CurseForgeApiResponse<Vec<CurseForgeSearchMod>>>(&response.text()?)?;

    Ok(payload
        .data
        .into_iter()
        .map(|mod_item| {
            let matched_index = mod_item
                .latest_files_indexes
                .iter()
                .find(|item| item.game_version_type_id == wow_context.version_type_id);
            let file_id = matched_index.map(|item| item.file_id);
            let source = AddonSourceRef::CurseForgeMod {
                mod_id: mod_item.id,
                file_id,
            };
            let install_hint = source.display_name();

            AddonSearchResult {
                provider: "curseforge",
                name: mod_item.name,
                summary: mod_item.summary,
                source,
                install_hint,
                website_url: mod_item.links.website_url,
                provider_project_id: Some(mod_item.id),
                provider_file_id: file_id,
                download_count: mod_item.download_count,
            }
        })
        .collect())
}

fn select_latest_curseforge_file(
    files: Vec<CurseForgeFile>,
    version_type_id: Option<u32>,
) -> AppResult<CurseForgeFile> {
    let mut candidates = files
        .into_iter()
        .filter(|file| file.is_available)
        .filter(|file| file.file_name.ends_with(".zip"))
        .filter(|file| {
            version_type_id.is_none_or(|version_type_id| {
                file_matches_curseforge_version_type(file, version_type_id)
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.file_date.cmp(&left.file_date));

    let Some(file) = candidates.into_iter().next() else {
        return Err(AppError::Validation(match version_type_id {
            Some(version_type_id) => format!(
                "CurseForge mod does not expose an available `.zip` file for version type `{version_type_id}`"
            ),
            None => "CurseForge mod does not expose an available `.zip` file".to_string(),
        }));
    };

    validate_curseforge_file(file)
}

fn ensure_curseforge_file_matches_version_type(
    file: &CurseForgeFile,
    version_type_id: u32,
) -> AppResult<()> {
    if file_matches_curseforge_version_type(file, version_type_id) {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "CurseForge file `{}` does not match target version type `{version_type_id}`",
            file.id
        )))
    }
}

fn file_matches_curseforge_version_type(file: &CurseForgeFile, version_type_id: u32) -> bool {
    file.sortable_game_versions
        .iter()
        .any(|item| item.game_version_type_id == version_type_id)
}

fn validate_curseforge_file(file: CurseForgeFile) -> AppResult<CurseForgeFile> {
    if !file.is_available {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` is not available for download",
            file.id
        )));
    }
    if !file.file_name.ends_with(".zip") {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` is not a `.zip` archive",
            file.file_name
        )));
    }
    if file.download_url.is_none() {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` does not provide a download URL",
            file.id
        )));
    }

    Ok(file)
}

fn select_github_release_asset<'a>(
    release: &'a GitHubRelease,
    requested_asset_name: Option<&str>,
) -> AppResult<&'a GitHubReleaseAsset> {
    if let Some(requested_asset_name) = requested_asset_name {
        return release
            .assets
            .iter()
            .find(|asset| asset.name.eq_ignore_ascii_case(requested_asset_name))
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "GitHub release asset `{requested_asset_name}` not found; available assets: {}",
                    release
                        .assets
                        .iter()
                        .map(|asset| asset.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            });
    }

    let zip_assets = release
        .assets
        .iter()
        .filter(|asset| asset.name.ends_with(".zip"))
        .collect::<Vec<_>>();
    match zip_assets.len() {
        0 => Err(AppError::Validation(
            "GitHub release does not contain a `.zip` asset".to_string(),
        )),
        1 => Ok(zip_assets[0]),
        _ => Err(AppError::Validation(format!(
            "GitHub release has multiple `.zip` assets; specify one with `github:owner/repo[#asset.zip]`: {}",
            zip_assets
                .into_iter()
                .map(|asset| asset.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
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

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct CurseForgeApiResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgePaginatedResponse<T> {
    data: T,
}

#[derive(Debug, Clone)]
struct CurseForgeWowContext {
    game_id: u32,
    version_type_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeGame {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeGameVersionType {
    id: u32,
    name: String,
    slug: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFile {
    id: u32,
    file_name: String,
    file_date: String,
    download_url: Option<String>,
    is_available: bool,
    #[serde(default)]
    sortable_game_versions: Vec<CurseForgeSortableGameVersion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeSortableGameVersion {
    game_version_type_id: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeSearchMod {
    id: u32,
    name: String,
    summary: Option<String>,
    download_count: u64,
    #[serde(default)]
    latest_files_indexes: Vec<CurseForgeFileIndex>,
    links: CurseForgeSearchModLinks,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeSearchModLinks {
    website_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFileIndex {
    file_id: u32,
    game_version_type_id: u32,
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    use super::{
        AddonSourceRef, CurseForgeFile, CurseForgeGameVersionType, CurseForgeSortableGameVersion,
        GitHubRelease, GitHubReleaseAsset, parse_curseforge_source, parse_github_source,
        select_curseforge_version_type, select_github_release_asset, select_latest_curseforge_file,
        validate_curseforge_file,
    };
    use crate::core::install::WowFlavor;

    #[derive(Debug, Deserialize, Serialize)]
    struct AddonSourceFixture {
        source: AddonSourceRef,
    }

    #[test]
    fn addon_source_ref_uses_canonical_provider_kind_names() {
        let github = AddonSourceFixture {
            source: AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: None,
            },
        };
        let curseforge = AddonSourceFixture {
            source: AddonSourceRef::CurseForgeMod {
                mod_id: 12345,
                file_id: None,
            },
        };

        assert!(
            toml::to_string(&github)
                .expect("github source toml")
                .contains("kind = \"github_release\"")
        );
        assert!(
            toml::to_string(&curseforge)
                .expect("curseforge source toml")
                .contains("kind = \"curseforge_mod\"")
        );
    }

    #[test]
    fn addon_source_ref_accepts_legacy_provider_kind_names() {
        let github: AddonSourceFixture = toml::from_str(
            r#"
source = { kind = "git_hub_release", owner = "owner", repo = "repo" }
"#,
        )
        .expect("legacy github source");
        let curseforge: AddonSourceFixture = toml::from_str(
            r#"
source = { kind = "curse_forge_mod", mod_id = 12345 }
"#,
        )
        .expect("legacy curseforge source");

        assert_eq!(
            github.source,
            AddonSourceRef::GitHubRelease {
                owner: "owner".to_string(),
                repo: "repo".to_string(),
                tag: None,
                asset_name: None,
            }
        );
        assert_eq!(
            curseforge.source,
            AddonSourceRef::CurseForgeMod {
                mod_id: 12345,
                file_id: None,
            }
        );
    }

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
    fn select_github_release_asset_requires_disambiguation() {
        let release = GitHubRelease {
            assets: vec![
                GitHubReleaseAsset {
                    name: "a.zip".to_string(),
                    browser_download_url: "https://example.com/a.zip".to_string(),
                },
                GitHubReleaseAsset {
                    name: "b.zip".to_string(),
                    browser_download_url: "https://example.com/b.zip".to_string(),
                },
            ],
        };

        let error = select_github_release_asset(&release, None).expect_err("ambiguous");
        assert!(error.to_string().contains("multiple `.zip` assets"));
    }

    #[test]
    fn select_github_release_asset_matches_explicit_asset() {
        let release = GitHubRelease {
            assets: vec![
                GitHubReleaseAsset {
                    name: "addon.zip".to_string(),
                    browser_download_url: "https://example.com/addon.zip".to_string(),
                },
                GitHubReleaseAsset {
                    name: "addon.txt".to_string(),
                    browser_download_url: "https://example.com/addon.txt".to_string(),
                },
            ],
        };

        let asset = select_github_release_asset(&release, Some("addon.zip")).expect("asset");
        assert_eq!(asset.name, "addon.zip");
    }

    #[test]
    fn select_latest_curseforge_file_prefers_newest_available_zip() {
        let file = select_latest_curseforge_file(
            vec![
                CurseForgeFile {
                    id: 1,
                    file_name: "addon-old.zip".to_string(),
                    file_date: "2026-04-01T12:00:00Z".to_string(),
                    download_url: Some("https://example.com/old.zip".to_string()),
                    is_available: true,
                    sortable_game_versions: vec![CurseForgeSortableGameVersion {
                        game_version_type_id: 517,
                    }],
                },
                CurseForgeFile {
                    id: 2,
                    file_name: "addon-new.zip".to_string(),
                    file_date: "2026-04-02T12:00:00Z".to_string(),
                    download_url: Some("https://example.com/new.zip".to_string()),
                    is_available: true,
                    sortable_game_versions: vec![CurseForgeSortableGameVersion {
                        game_version_type_id: 517,
                    }],
                },
                CurseForgeFile {
                    id: 3,
                    file_name: "addon.txt".to_string(),
                    file_date: "2026-04-03T12:00:00Z".to_string(),
                    download_url: Some("https://example.com/skip.txt".to_string()),
                    is_available: true,
                    sortable_game_versions: vec![CurseForgeSortableGameVersion {
                        game_version_type_id: 517,
                    }],
                },
            ],
            None,
        )
        .expect("latest file");

        assert_eq!(file.id, 2);
        assert_eq!(file.file_name, "addon-new.zip");
    }

    #[test]
    fn select_latest_curseforge_file_filters_by_version_type() {
        let file = select_latest_curseforge_file(
            vec![
                CurseForgeFile {
                    id: 1,
                    file_name: "addon-retail.zip".to_string(),
                    file_date: "2026-04-01T12:00:00Z".to_string(),
                    download_url: Some("https://example.com/retail.zip".to_string()),
                    is_available: true,
                    sortable_game_versions: vec![CurseForgeSortableGameVersion {
                        game_version_type_id: 517,
                    }],
                },
                CurseForgeFile {
                    id: 2,
                    file_name: "addon-classic.zip".to_string(),
                    file_date: "2026-04-02T12:00:00Z".to_string(),
                    download_url: Some("https://example.com/classic.zip".to_string()),
                    is_available: true,
                    sortable_game_versions: vec![CurseForgeSortableGameVersion {
                        game_version_type_id: 775,
                    }],
                },
            ],
            Some(517),
        )
        .expect("latest file");

        assert_eq!(file.id, 1);
    }

    #[test]
    fn select_curseforge_version_type_matches_retail_slug() {
        let version_type = select_curseforge_version_type(
            &[
                CurseForgeGameVersionType {
                    id: 775,
                    name: "WoW Classic".to_string(),
                    slug: "wow_classic".to_string(),
                },
                CurseForgeGameVersionType {
                    id: 517,
                    name: "WoW Retail".to_string(),
                    slug: "wow_retail".to_string(),
                },
            ],
            WowFlavor::Retail,
        )
        .expect("version type");

        assert_eq!(version_type.id, 517);
    }

    #[test]
    fn validate_curseforge_file_rejects_missing_download_url() {
        let error = validate_curseforge_file(CurseForgeFile {
            id: 1,
            file_name: "addon.zip".to_string(),
            file_date: "2026-04-02T12:00:00Z".to_string(),
            download_url: None,
            is_available: true,
            sortable_game_versions: Vec::new(),
        })
        .expect_err("missing download url");

        assert!(error.to_string().contains("download URL"));
    }
}
