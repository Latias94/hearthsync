use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::{AddonProvider, AddonSourceRef as DomainAddonSourceRef};
use crate::core::app::AddonDependencyResolutionCapabilityValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonSourceKindResult {
    LocalArchive,
    HttpArchive,
    CurseForgeMod,
    GitHubRelease,
    WagoAddon,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonSourceResult {
    pub kind: AddonSourceKindResult,
    pub display_name: String,
    pub dependency_resolution_capability: AddonDependencyResolutionCapabilityValue,
    pub local_archive_path: Option<PathBuf>,
    pub url: Option<String>,
    pub mod_id: Option<u32>,
    pub file_id: Option<u32>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub tag: Option<String>,
    pub asset_name: Option<String>,
    pub project_id: Option<String>,
    pub release_id: Option<String>,
}

impl AddonSourceResult {
    pub(crate) fn from_domain_with_provider<P>(value: DomainAddonSourceRef, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let display_name = value.display_name();
        let dependency_resolution_capability =
            AddonDependencyResolutionCapabilityValue::from_domain(
                provider.dependency_resolution_capability(&value),
            );

        match value {
            DomainAddonSourceRef::LocalArchive { path } => Self {
                kind: AddonSourceKindResult::LocalArchive,
                display_name,
                dependency_resolution_capability,
                local_archive_path: Some(path),
                url: None,
                mod_id: None,
                file_id: None,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
                project_id: None,
                release_id: None,
            },
            DomainAddonSourceRef::HttpArchive { url } => Self {
                kind: AddonSourceKindResult::HttpArchive,
                display_name,
                dependency_resolution_capability,
                local_archive_path: None,
                url: Some(url),
                mod_id: None,
                file_id: None,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
                project_id: None,
                release_id: None,
            },
            DomainAddonSourceRef::CurseForgeMod { mod_id, file_id } => Self {
                kind: AddonSourceKindResult::CurseForgeMod,
                display_name,
                dependency_resolution_capability,
                local_archive_path: None,
                url: None,
                mod_id: Some(mod_id),
                file_id,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
                project_id: None,
                release_id: None,
            },
            DomainAddonSourceRef::GitHubRelease {
                owner,
                repo,
                tag,
                asset_name,
            } => Self {
                kind: AddonSourceKindResult::GitHubRelease,
                display_name,
                dependency_resolution_capability,
                local_archive_path: None,
                url: None,
                mod_id: None,
                file_id: None,
                owner: Some(owner),
                repo: Some(repo),
                tag,
                asset_name,
                project_id: None,
                release_id: None,
            },
            DomainAddonSourceRef::WagoAddon {
                project_id,
                release_id,
            } => Self {
                kind: AddonSourceKindResult::WagoAddon,
                display_name,
                dependency_resolution_capability,
                local_archive_path: None,
                url: None,
                mod_id: None,
                file_id: None,
                owner: None,
                repo: None,
                tag: None,
                asset_name: None,
                project_id: Some(project_id),
                release_id,
            },
        }
    }
}
