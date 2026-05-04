use std::path::Path;

use super::cache::guess_archive_name_from_url;
use super::curseforge::{
    remote_validators_for_curseforge_file, resolve_curseforge_file_with_client,
};
use super::github::{
    fetch_github_release_with_client, fetch_github_releases_with_client,
    remote_validators_for_github_asset, select_github_release, select_github_release_asset,
};
use super::http::HttpClient;
use super::materialize::{
    ResolvedDownloadArtifact, materialize_downloaded_archive, materialize_http_archive_source,
    materialize_local_archive_source,
};
use super::parse::{parse_curseforge_source, parse_github_source};
use super::source::{canonicalize_local_archive_path, validate_absolute_local_archive_source_path};
use super::source_adapter::{
    curseforge_release_type_limit, github_allows_prerelease, resolve_source_dependencies_impl,
    search_addons_impl,
};
use super::{
    AddonDependencyResolutionCapability, AddonProviderContext, AddonProviderOptions,
    AddonSearchRequest, AddonSearchResult, AddonSourceRef, MaterializedAddonSource,
    ResolveAddonDependenciesRequest, ResolvedAddonDependencies,
};
use crate::core::boundary_validation::is_http_url;
use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonSourceFamily {
    LocalArchive,
    HttpArchive,
    CurseForgeMod,
    GitHubRelease,
}

impl AddonSourceFamily {
    pub fn from_source(source: &AddonSourceRef) -> Self {
        match source {
            AddonSourceRef::LocalArchive { .. } => Self::LocalArchive,
            AddonSourceRef::HttpArchive { .. } => Self::HttpArchive,
            AddonSourceRef::CurseForgeMod { .. } => Self::CurseForgeMod,
            AddonSourceRef::GitHubRelease { .. } => Self::GitHubRelease,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonProviderSourceCapability {
    pub source_family: AddonSourceFamily,
    pub provider_id: &'static str,
    pub provider_name: &'static str,
    pub input_prefix: Option<&'static str>,
    pub can_parse_input: bool,
    pub can_materialize: bool,
    pub can_search: bool,
    pub dependency_resolution: AddonDependencyResolutionCapability,
    pub supports_release_channel: bool,
    pub supports_prerelease: bool,
    pub supports_version_pin: bool,
    pub supports_file_id_pin: bool,
    pub supports_remote_cache_validators: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AddonProviderRegistry;

impl AddonProviderRegistry {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn source_capabilities(&self) -> Vec<AddonProviderSourceCapability> {
        vec![
            AddonProviderSourceCapability {
                source_family: AddonSourceFamily::LocalArchive,
                provider_id: "local",
                provider_name: "Local archive",
                input_prefix: None,
                can_parse_input: true,
                can_materialize: true,
                can_search: false,
                dependency_resolution: AddonDependencyResolutionCapability::Unsupported,
                supports_release_channel: false,
                supports_prerelease: false,
                supports_version_pin: false,
                supports_file_id_pin: false,
                supports_remote_cache_validators: false,
            },
            AddonProviderSourceCapability {
                source_family: AddonSourceFamily::HttpArchive,
                provider_id: "http",
                provider_name: "HTTP archive",
                input_prefix: Some("http:// or https://"),
                can_parse_input: true,
                can_materialize: true,
                can_search: false,
                dependency_resolution: AddonDependencyResolutionCapability::Unsupported,
                supports_release_channel: false,
                supports_prerelease: false,
                supports_version_pin: false,
                supports_file_id_pin: false,
                supports_remote_cache_validators: true,
            },
            AddonProviderSourceCapability {
                source_family: AddonSourceFamily::CurseForgeMod,
                provider_id: "curseforge",
                provider_name: "CurseForge",
                input_prefix: Some("curseforge:"),
                can_parse_input: true,
                can_materialize: true,
                can_search: true,
                dependency_resolution: AddonDependencyResolutionCapability::missing_required_only(),
                supports_release_channel: true,
                supports_prerelease: true,
                supports_version_pin: false,
                supports_file_id_pin: true,
                supports_remote_cache_validators: true,
            },
            AddonProviderSourceCapability {
                source_family: AddonSourceFamily::GitHubRelease,
                provider_id: "github",
                provider_name: "GitHub Releases",
                input_prefix: Some("github:"),
                can_parse_input: true,
                can_materialize: true,
                can_search: false,
                dependency_resolution: AddonDependencyResolutionCapability::Unsupported,
                supports_release_channel: true,
                supports_prerelease: true,
                supports_version_pin: true,
                supports_file_id_pin: false,
                supports_remote_cache_validators: true,
            },
        ]
    }

    pub(super) fn dependency_resolution_capability(
        &self,
        source: &AddonSourceRef,
    ) -> AddonDependencyResolutionCapability {
        match AddonSourceFamily::from_source(source) {
            AddonSourceFamily::CurseForgeMod => {
                AddonDependencyResolutionCapability::missing_required_only()
            }
            AddonSourceFamily::LocalArchive
            | AddonSourceFamily::HttpArchive
            | AddonSourceFamily::GitHubRelease => AddonDependencyResolutionCapability::Unsupported,
        }
    }

    pub(super) fn materialize_source_input(
        &self,
        http_client: &impl HttpClient,
        source: &str,
        stage_root: &Path,
        context: AddonProviderContext<'_>,
        options: &AddonProviderOptions,
    ) -> AppResult<MaterializedAddonSource> {
        let source_ref = self.parse_source_input(source)?;
        self.materialize_source_ref(http_client, &source_ref, stage_root, context, options)
    }

    pub(super) fn materialize_source_ref(
        &self,
        http_client: &impl HttpClient,
        source: &AddonSourceRef,
        stage_root: &Path,
        context: AddonProviderContext<'_>,
        options: &AddonProviderOptions,
    ) -> AppResult<MaterializedAddonSource> {
        match source {
            AddonSourceRef::LocalArchive { path } => materialize_local_archive_source(source, path),
            AddonSourceRef::HttpArchive { url } => materialize_http_archive_source(
                http_client,
                source,
                url,
                stage_root,
                context.cancellation,
                context.download_progress,
                options,
            ),
            AddonSourceRef::CurseForgeMod { mod_id, file_id } => {
                let artifact =
                    self.resolve_curseforge_artifact(http_client, *mod_id, *file_id, context)?;
                let archive_path = materialize_downloaded_archive(
                    http_client,
                    artifact,
                    stage_root,
                    context.cancellation,
                    context.download_progress,
                    options,
                )?;
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
                let artifact = self.resolve_github_artifact(
                    http_client,
                    owner,
                    repo,
                    tag.as_deref(),
                    asset_name.as_deref(),
                    context,
                )?;
                let archive_path = materialize_downloaded_archive(
                    http_client,
                    artifact,
                    stage_root,
                    context.cancellation,
                    context.download_progress,
                    options,
                )?;
                Ok(MaterializedAddonSource {
                    source_ref: source.clone(),
                    archive_path,
                })
            }
        }
    }

    pub(super) fn resolve_addon_dependencies(
        &self,
        http_client: &impl HttpClient,
        request: ResolveAddonDependenciesRequest<'_>,
    ) -> AppResult<ResolvedAddonDependencies> {
        resolve_source_dependencies_impl(http_client, request.source, request.context)
    }

    pub(super) fn search_addons(
        &self,
        http_client: &impl HttpClient,
        request: AddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        search_addons_impl(http_client, request.query, request.flavor, request.limit)
    }

    fn parse_source_input(&self, source: &str) -> AppResult<AddonSourceRef> {
        if let Some(source_ref) = parse_curseforge_source(source)? {
            return Ok(source_ref);
        }

        if let Some(source_ref) = parse_github_source(source)? {
            return Ok(source_ref);
        }

        if is_http_url(source) {
            return Ok(AddonSourceRef::HttpArchive {
                url: source.to_string(),
            });
        }

        let path = canonicalize_local_archive_path(Path::new(source))?;
        Ok(AddonSourceRef::LocalArchive { path })
    }

    fn resolve_curseforge_artifact(
        &self,
        http_client: &impl HttpClient,
        mod_id: u32,
        file_id: Option<u32>,
        context: AddonProviderContext<'_>,
    ) -> AppResult<ResolvedDownloadArtifact> {
        let file = resolve_curseforge_file_with_client(
            http_client,
            mod_id,
            file_id,
            context.target_flavor,
            curseforge_release_type_limit(context.resolution_policy),
        )?;
        let download_url = file.download_url.clone().ok_or_else(|| {
            AppError::Validation(format!(
                "CurseForge file `{}` does not provide a download URL",
                file.id
            ))
        })?;

        Ok(ResolvedDownloadArtifact {
            cache_source_ref: AddonSourceRef::CurseForgeMod {
                mod_id,
                file_id: Some(file.id),
            },
            download_url,
            archive_name: file.file_name.clone(),
            headers: Vec::new(),
            remote_validators: remote_validators_for_curseforge_file(&file),
        })
    }

    fn resolve_github_artifact(
        &self,
        http_client: &impl HttpClient,
        owner: &str,
        repo: &str,
        tag: Option<&str>,
        asset_name: Option<&str>,
        context: AddonProviderContext<'_>,
    ) -> AppResult<ResolvedDownloadArtifact> {
        let release = match tag {
            Some(tag) => fetch_github_release_with_client(http_client, owner, repo, Some(tag))?,
            None if github_allows_prerelease(context.resolution_policy) => {
                let releases = fetch_github_releases_with_client(http_client, owner, repo)?;
                select_github_release(&releases, true)?.clone()
            }
            None => fetch_github_release_with_client(http_client, owner, repo, None)?,
        };
        let asset = select_github_release_asset(&release, asset_name)?;

        Ok(ResolvedDownloadArtifact {
            cache_source_ref: AddonSourceRef::GitHubRelease {
                owner: owner.to_string(),
                repo: repo.to_string(),
                tag: Some(release.tag_name.clone()),
                asset_name: Some(asset.name.clone()),
            },
            download_url: asset.browser_download_url.clone(),
            archive_name: asset.name.clone(),
            headers: Vec::new(),
            remote_validators: remote_validators_for_github_asset(asset),
        })
    }
}

pub(super) fn http_archive_artifact_name(url: &str) -> String {
    guess_archive_name_from_url(url).unwrap_or_else(|| "downloaded-addon.zip".to_string())
}

pub(super) fn validate_persisted_local_source(path: &Path) -> AppResult<()> {
    validate_absolute_local_archive_source_path(path)
}
