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
use super::source::{
    addon_source_input_is_local_archive, canonicalize_local_archive_path, source_kind_label,
    validate_absolute_local_archive_source_path,
};
use super::source_adapter::{
    curseforge_release_type_limit, github_allows_prerelease, resolve_source_dependencies_impl,
    search_addons_impl,
};
use super::{
    AddonDependencyResolutionCapability, AddonProviderContext, AddonProviderOptions,
    AddonSearchRequest, AddonSearchResult, AddonSourceRef, AddonSourceResolutionPolicy,
    AppliedAddonSourcePolicy, ApplyAddonSourcePolicyRequest, MaterializedAddonSource,
    ResolveAddonDependenciesRequest, ResolvedAddonDependencies,
};
use crate::core::addon::policy::AddonPolicyPin;
use crate::core::boundary_validation::is_http_url;
use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AddonSourceFamily {
    id: &'static str,
}

impl AddonSourceFamily {
    pub const LOCAL_ARCHIVE: Self = Self {
        id: "local_archive",
    };
    pub const HTTP_ARCHIVE: Self = Self { id: "http_archive" };
    pub const CURSEFORGE_MOD: Self = Self {
        id: "curseforge_mod",
    };
    pub const GITHUB_RELEASE: Self = Self {
        id: "github_release",
    };

    pub const fn from_static_id(id: &'static str) -> Self {
        Self { id }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub fn from_source(source: &AddonSourceRef) -> Self {
        match source {
            AddonSourceRef::LocalArchive { .. } => Self::LOCAL_ARCHIVE,
            AddonSourceRef::HttpArchive { .. } => Self::HTTP_ARCHIVE,
            AddonSourceRef::CurseForgeMod { .. } => Self::CURSEFORGE_MOD,
            AddonSourceRef::GitHubRelease { .. } => Self::GITHUB_RELEASE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddonProviderOperationCapabilities {
    pub can_parse_input: bool,
    pub can_materialize: bool,
    pub can_search: bool,
    pub dependency_resolution: AddonDependencyResolutionCapability,
    pub supports_remote_cache_validators: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddonProviderPolicyCapabilities {
    pub supports_release_channel: bool,
    pub supports_prerelease: bool,
    pub supports_version_pin: bool,
    pub supports_file_id_pin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddonProviderDescriptor {
    pub source_family: AddonSourceFamily,
    pub provider_id: &'static str,
    pub provider_name: &'static str,
    pub input_prefix: Option<&'static str>,
    pub operations: AddonProviderOperationCapabilities,
    pub policy: AddonProviderPolicyCapabilities,
}

impl AddonProviderDescriptor {
    pub fn source_capability(self) -> AddonProviderSourceCapability {
        AddonProviderSourceCapability {
            source_family: self.source_family,
            provider_id: self.provider_id,
            provider_name: self.provider_name,
            input_prefix: self.input_prefix,
            can_parse_input: self.operations.can_parse_input,
            can_materialize: self.operations.can_materialize,
            can_search: self.operations.can_search,
            dependency_resolution: self.operations.dependency_resolution,
            supports_release_channel: self.policy.supports_release_channel,
            supports_prerelease: self.policy.supports_prerelease,
            supports_version_pin: self.policy.supports_version_pin,
            supports_file_id_pin: self.policy.supports_file_id_pin,
            supports_remote_cache_validators: self.operations.supports_remote_cache_validators,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuiltinAddonProviderAdapter {
    LocalArchive,
    HttpArchive,
    CurseForge,
    GitHub,
}

const BUILTIN_PROVIDER_ADAPTERS: &[BuiltinAddonProviderAdapter] = &[
    BuiltinAddonProviderAdapter::LocalArchive,
    BuiltinAddonProviderAdapter::HttpArchive,
    BuiltinAddonProviderAdapter::CurseForge,
    BuiltinAddonProviderAdapter::GitHub,
];

impl BuiltinAddonProviderAdapter {
    fn descriptor(self) -> AddonProviderDescriptor {
        match self {
            Self::LocalArchive => AddonProviderDescriptor {
                source_family: AddonSourceFamily::LOCAL_ARCHIVE,
                provider_id: "local",
                provider_name: "Local archive",
                input_prefix: None,
                operations: AddonProviderOperationCapabilities {
                    can_parse_input: true,
                    can_materialize: true,
                    can_search: false,
                    dependency_resolution: AddonDependencyResolutionCapability::Unsupported,
                    supports_remote_cache_validators: false,
                },
                policy: AddonProviderPolicyCapabilities {
                    supports_release_channel: false,
                    supports_prerelease: false,
                    supports_version_pin: false,
                    supports_file_id_pin: false,
                },
            },
            Self::HttpArchive => AddonProviderDescriptor {
                source_family: AddonSourceFamily::HTTP_ARCHIVE,
                provider_id: "http",
                provider_name: "HTTP archive",
                input_prefix: Some("http:// or https://"),
                operations: AddonProviderOperationCapabilities {
                    can_parse_input: true,
                    can_materialize: true,
                    can_search: false,
                    dependency_resolution: AddonDependencyResolutionCapability::Unsupported,
                    supports_remote_cache_validators: true,
                },
                policy: AddonProviderPolicyCapabilities {
                    supports_release_channel: false,
                    supports_prerelease: false,
                    supports_version_pin: false,
                    supports_file_id_pin: false,
                },
            },
            Self::CurseForge => AddonProviderDescriptor {
                source_family: AddonSourceFamily::CURSEFORGE_MOD,
                provider_id: "curseforge",
                provider_name: "CurseForge",
                input_prefix: Some("curseforge:"),
                operations: AddonProviderOperationCapabilities {
                    can_parse_input: true,
                    can_materialize: true,
                    can_search: true,
                    dependency_resolution:
                        AddonDependencyResolutionCapability::missing_required_only(),
                    supports_remote_cache_validators: true,
                },
                policy: AddonProviderPolicyCapabilities {
                    supports_release_channel: true,
                    supports_prerelease: true,
                    supports_version_pin: false,
                    supports_file_id_pin: true,
                },
            },
            Self::GitHub => AddonProviderDescriptor {
                source_family: AddonSourceFamily::GITHUB_RELEASE,
                provider_id: "github",
                provider_name: "GitHub Releases",
                input_prefix: Some("github:"),
                operations: AddonProviderOperationCapabilities {
                    can_parse_input: true,
                    can_materialize: true,
                    can_search: false,
                    dependency_resolution: AddonDependencyResolutionCapability::Unsupported,
                    supports_remote_cache_validators: true,
                },
                policy: AddonProviderPolicyCapabilities {
                    supports_release_channel: true,
                    supports_prerelease: true,
                    supports_version_pin: true,
                    supports_file_id_pin: false,
                },
            },
        }
    }

    fn matches_source(self, source: &AddonSourceRef) -> bool {
        self.descriptor().source_family == AddonSourceFamily::from_source(source)
    }

    fn accepts_source_input(self, source: &str) -> bool {
        match self {
            Self::LocalArchive => addon_source_input_is_local_archive(source),
            Self::HttpArchive => is_http_url(source),
            Self::CurseForge => source.starts_with("curseforge:"),
            Self::GitHub => source.starts_with("github:"),
        }
    }

    fn parse_source_input(self, source: &str) -> AppResult<Option<AddonSourceRef>> {
        if !self.descriptor().operations.can_parse_input || !self.accepts_source_input(source) {
            return Ok(None);
        }

        match self {
            Self::LocalArchive => {
                let path = canonicalize_local_archive_path(Path::new(source))?;
                Ok(Some(AddonSourceRef::LocalArchive { path }))
            }
            Self::HttpArchive => Ok(Some(AddonSourceRef::HttpArchive {
                url: source.to_string(),
            })),
            Self::CurseForge => parse_curseforge_source(source),
            Self::GitHub => parse_github_source(source),
        }
    }

    fn materialize_source_ref(
        self,
        registry: AddonProviderRegistry,
        http_client: &impl HttpClient,
        source: &AddonSourceRef,
        stage_root: &Path,
        context: AddonProviderContext<'_>,
        options: &AddonProviderOptions,
    ) -> AppResult<MaterializedAddonSource> {
        match (self, source) {
            (Self::LocalArchive, AddonSourceRef::LocalArchive { path }) => {
                materialize_local_archive_source(source, path)
            }
            (Self::HttpArchive, AddonSourceRef::HttpArchive { url }) => {
                materialize_http_archive_source(
                    http_client,
                    source,
                    url,
                    stage_root,
                    context.cancellation,
                    context.download_progress,
                    options,
                )
            }
            (Self::CurseForge, AddonSourceRef::CurseForgeMod { mod_id, file_id }) => {
                let artifact = registry.resolve_curseforge_artifact(
                    http_client,
                    *mod_id,
                    *file_id,
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
            (
                Self::GitHub,
                AddonSourceRef::GitHubRelease {
                    owner,
                    repo,
                    tag,
                    asset_name,
                },
            ) => {
                let artifact = registry.resolve_github_artifact(
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
            _ => Err(AppError::Validation(format!(
                "addon source `{}` is not handled by provider `{}`",
                source.display_name(),
                self.descriptor().provider_id
            ))),
        }
    }

    fn resolve_addon_dependencies(
        self,
        http_client: &impl HttpClient,
        request: ResolveAddonDependenciesRequest<'_>,
    ) -> AppResult<ResolvedAddonDependencies> {
        match self {
            Self::CurseForge => {
                resolve_source_dependencies_impl(http_client, request.source, request.context)
            }
            Self::LocalArchive | Self::HttpArchive | Self::GitHub => {
                Err(unsupported_dependency_error(request.source))
            }
        }
    }

    fn apply_source_policy(
        self,
        request: ApplyAddonSourcePolicyRequest<'_>,
    ) -> AppResult<AppliedAddonSourcePolicy> {
        let descriptor = self.descriptor();
        validate_resolution_policy_support(descriptor, request.source, request.resolution_policy)?;
        let source = match request.pin {
            Some(pin) => self.apply_source_pin(request.source, pin)?,
            None => request.source.clone(),
        };

        Ok(AppliedAddonSourcePolicy {
            source,
            resolution_policy: request.resolution_policy,
        })
    }

    fn apply_source_pin(
        self,
        source: &AddonSourceRef,
        pin: &AddonPolicyPin,
    ) -> AppResult<AddonSourceRef> {
        let descriptor = self.descriptor();
        match (self, source, pin) {
            (
                Self::CurseForge,
                AddonSourceRef::CurseForgeMod { mod_id, .. },
                AddonPolicyPin::FileId { value },
            ) if descriptor.policy.supports_file_id_pin => Ok(AddonSourceRef::CurseForgeMod {
                mod_id: *mod_id,
                file_id: Some(*value),
            }),
            (
                Self::GitHub,
                AddonSourceRef::GitHubRelease {
                    owner,
                    repo,
                    asset_name,
                    ..
                },
                AddonPolicyPin::Version { value },
            ) if descriptor.policy.supports_version_pin => Ok(AddonSourceRef::GitHubRelease {
                owner: owner.clone(),
                repo: repo.clone(),
                tag: Some(value.clone()),
                asset_name: asset_name.clone(),
            }),
            (
                Self::CurseForge,
                AddonSourceRef::CurseForgeMod { .. },
                AddonPolicyPin::Version { .. },
            ) => Err(unsupported_policy_error(
                descriptor,
                source,
                "version pin",
                "addon policy pinned version is not supported for CurseForge sources yet",
            )),
            (Self::GitHub, AddonSourceRef::GitHubRelease { .. }, AddonPolicyPin::FileId { .. }) => {
                Err(unsupported_policy_error(
                    descriptor,
                    source,
                    "file id pin",
                    "addon policy pinned file id is not supported for GitHub release sources",
                ))
            }
            (Self::LocalArchive | Self::HttpArchive, _, _) => Err(unsupported_policy_error(
                descriptor,
                source,
                "pinning",
                "addon policy pinning is only supported for provider-backed addon sources",
            )),
            _ => Err(AppError::Validation(format!(
                "addon source `{}` is not handled by provider `{}`",
                source.display_name(),
                descriptor.provider_id
            ))),
        }
    }

    fn search_addons(
        self,
        http_client: &impl HttpClient,
        request: AddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        match self {
            Self::CurseForge => {
                search_addons_impl(http_client, request.query, request.flavor, request.limit)
            }
            Self::LocalArchive | Self::HttpArchive | Self::GitHub => Ok(Vec::new()),
        }
    }
}

impl AddonProviderRegistry {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn provider_descriptors(&self) -> Vec<AddonProviderDescriptor> {
        self.provider_adapters()
            .iter()
            .map(|adapter| adapter.descriptor())
            .collect()
    }

    pub(super) fn dependency_resolution_capability(
        &self,
        source: &AddonSourceRef,
    ) -> AddonDependencyResolutionCapability {
        self.provider_for_source(source)
            .map(|adapter| adapter.descriptor().operations.dependency_resolution)
            .unwrap_or(AddonDependencyResolutionCapability::Unsupported)
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
        self.provider_for_source(source)?.materialize_source_ref(
            *self,
            http_client,
            source,
            stage_root,
            context,
            options,
        )
    }

    pub(super) fn resolve_addon_dependencies(
        &self,
        http_client: &impl HttpClient,
        request: ResolveAddonDependenciesRequest<'_>,
    ) -> AppResult<ResolvedAddonDependencies> {
        self.provider_for_source(request.source)?
            .resolve_addon_dependencies(http_client, request)
    }

    pub(super) fn apply_source_policy(
        &self,
        request: ApplyAddonSourcePolicyRequest<'_>,
    ) -> AppResult<AppliedAddonSourcePolicy> {
        self.provider_for_source(request.source)?
            .apply_source_policy(request)
    }

    pub(super) fn search_addons(
        &self,
        http_client: &impl HttpClient,
        request: AddonSearchRequest<'_>,
    ) -> AppResult<Vec<AddonSearchResult>> {
        let mut results = Vec::new();
        for adapter in self.provider_adapters() {
            if !adapter.descriptor().operations.can_search {
                continue;
            }
            results.extend(adapter.search_addons(http_client, request)?);
        }
        Ok(results)
    }

    fn parse_source_input(&self, source: &str) -> AppResult<AddonSourceRef> {
        for adapter in self.provider_adapters() {
            if let Some(source_ref) = adapter.parse_source_input(source)? {
                return Ok(source_ref);
            }
        }

        Err(AppError::Validation(format!(
            "addon source input is not supported by any registered provider: {source}"
        )))
    }

    fn provider_adapters(&self) -> &'static [BuiltinAddonProviderAdapter] {
        BUILTIN_PROVIDER_ADAPTERS
    }

    fn provider_for_source(
        &self,
        source: &AddonSourceRef,
    ) -> AppResult<BuiltinAddonProviderAdapter> {
        self.provider_adapters()
            .iter()
            .copied()
            .find(|adapter| adapter.matches_source(source))
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "addon source family `{}` is not handled by any registered provider",
                    AddonSourceFamily::from_source(source).id()
                ))
            })
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

fn unsupported_dependency_error(source: &AddonSourceRef) -> AppError {
    AppError::Validation(format!(
        "addon dependency installation is currently only supported for CurseForge sources, but `{}` uses `{}`",
        source.display_name(),
        source_kind_label(source),
    ))
}

fn validate_resolution_policy_support(
    descriptor: AddonProviderDescriptor,
    source: &AddonSourceRef,
    policy: AddonSourceResolutionPolicy,
) -> AppResult<()> {
    if policy.release_channel.is_some() && !descriptor.policy.supports_release_channel {
        return Err(unsupported_policy_error(
            descriptor,
            source,
            "release channel policy",
            "addon policy release channel is not supported for this source provider",
        ));
    }

    if policy.allow_prerelease.is_some() && !descriptor.policy.supports_prerelease {
        return Err(unsupported_policy_error(
            descriptor,
            source,
            "prerelease policy",
            "addon policy prerelease selection is not supported for this source provider",
        ));
    }

    Ok(())
}

fn unsupported_policy_error(
    descriptor: AddonProviderDescriptor,
    source: &AddonSourceRef,
    capability: &str,
    message: &str,
) -> AppError {
    AppError::Validation(format!(
        "{message} (provider: {}, source_family: {}, source: {}, capability: {capability})",
        descriptor.provider_id,
        descriptor.source_family.id(),
        source.display_name(),
    ))
}
