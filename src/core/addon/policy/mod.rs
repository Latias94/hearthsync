mod storage;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

use super::provider::{AddonProvider, AddonSourceResolutionPolicy, ApplyAddonSourcePolicyRequest};
use super::{AddonSourceRef, AddonStatePaths, TrackedAddonPackage};

pub(crate) use self::storage::load_addon_policy_state;
pub use self::storage::{inspect_addon_policy, policy_path, remove_addon_policy, set_addon_policy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonReleaseChannel {
    Stable,
    Beta,
    Alpha,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddonPolicyPin {
    Version { value: String },
    FileId { value: u32 },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AddonPolicyPackageView {
    pub package_id: String,
    pub package_name: Option<String>,
    pub addon_directories: Vec<String>,
    pub tracked: bool,
    pub ignored: Option<bool>,
    pub pin: Option<AddonPolicyPin>,
    pub release_channel: Option<AddonReleaseChannel>,
    pub allow_prerelease: Option<bool>,
    pub install_dependencies: Option<bool>,
}

impl AddonPolicyPackageView {
    pub(crate) fn from_entry(
        entry: &AddonPolicyPackageEntry,
        tracked_package: Option<&TrackedAddonPackage>,
    ) -> Self {
        let mut addon_directories = tracked_package
            .map(|package| {
                package
                    .addons
                    .iter()
                    .map(|addon| addon.directory_name.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        addon_directories.sort();

        Self {
            package_id: entry.package_id.clone(),
            package_name: tracked_package.and_then(policy_display_name),
            addon_directories,
            tracked: tracked_package.is_some(),
            ignored: entry.ignored,
            pin: entry.pin.clone(),
            release_channel: entry.release_channel,
            allow_prerelease: entry.allow_prerelease,
            install_dependencies: entry.install_dependencies,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonPolicyInspection {
    pub policy_path: PathBuf,
    pub package_count: usize,
    pub packages: Vec<AddonPolicyPackageView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonPolicyMutationResult {
    pub policy_path: PathBuf,
    pub package_count: usize,
    pub package_id: String,
    pub entry_removed: bool,
    pub package: Option<AddonPolicyPackageView>,
}

#[derive(Debug, Clone)]
pub struct SetAddonPolicyRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub package: String,
    pub ignored: Option<bool>,
    pub pinned_version: Option<String>,
    pub pinned_file_id: Option<u32>,
    pub release_channel: Option<AddonReleaseChannel>,
    pub allow_prerelease: Option<bool>,
    pub install_dependencies: Option<bool>,
}

impl SetAddonPolicyRequest {
    pub(crate) fn has_updates(&self) -> bool {
        self.ignored.is_some()
            || self.pinned_version.is_some()
            || self.pinned_file_id.is_some()
            || self.release_channel.is_some()
            || self.allow_prerelease.is_some()
            || self.install_dependencies.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct RemoveAddonPolicyRequest {
    pub installation: DetectedFlavorInstallation,
    pub(crate) state_paths: AddonStatePaths,
    pub package: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AddonPolicyState {
    pub(crate) schema_version: u32,
    pub(crate) updated_at: String,
    pub(crate) packages: Vec<AddonPolicyPackageEntry>,
}

impl Default for AddonPolicyState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            updated_at: String::new(),
            packages: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct AddonPolicyPackageEntry {
    pub(crate) package_id: String,
    #[serde(default)]
    pub(crate) ignored: Option<bool>,
    #[serde(default)]
    pub(crate) pin: Option<AddonPolicyPin>,
    #[serde(default)]
    pub(crate) release_channel: Option<AddonReleaseChannel>,
    #[serde(default)]
    pub(crate) allow_prerelease: Option<bool>,
    #[serde(default)]
    pub(crate) install_dependencies: Option<bool>,
}

impl AddonPolicyPackageEntry {
    pub(crate) fn new(package_id: String) -> Self {
        Self {
            package_id,
            ignored: None,
            pin: None,
            release_channel: None,
            allow_prerelease: None,
            install_dependencies: None,
        }
    }
}

pub(crate) fn policy_display_name(package: &TrackedAddonPackage) -> Option<String> {
    package
        .metadata
        .as_ref()
        .and_then(|value| value.package_name.clone())
        .or_else(|| {
            package
                .addons
                .iter()
                .filter_map(|addon| addon.title.clone())
                .map(|value| value.trim().to_string())
                .find(|value| !value.is_empty())
        })
        .or_else(|| Some(package.package_id.clone()))
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AddonUpdatePolicySnapshot {
    entries: BTreeMap<String, AddonPolicyPackageEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderAddonUpdatePolicy {
    pub(crate) effective_source: AddonSourceRef,
    pub(crate) resolution_policy: AddonSourceResolutionPolicy,
    pub(crate) install_dependencies: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndexedAddonUpdatePolicy {
    pub(crate) ignored: bool,
    pub(crate) install_dependencies: bool,
}

impl AddonUpdatePolicySnapshot {
    pub(crate) fn load(
        installation: &DetectedFlavorInstallation,
        state_paths: &AddonStatePaths,
    ) -> AppResult<Self> {
        let state = load_addon_policy_state(installation, state_paths)?;
        Ok(Self {
            entries: state
                .packages
                .into_iter()
                .map(|entry| (normalize_package_key(&entry.package_id), entry))
                .collect(),
        })
    }

    pub(crate) fn provider_update_policy<P>(
        &self,
        provider: &P,
        package: &TrackedAddonPackage,
    ) -> AppResult<ProviderAddonUpdatePolicy>
    where
        P: AddonProvider + ?Sized,
    {
        let entry = self.entry_for_package(package);
        let resolution_policy = provider_resolution_policy(entry);
        let applied = provider.apply_source_policy(ApplyAddonSourcePolicyRequest {
            source: &package.source,
            pin: entry.and_then(|value| value.pin.as_ref()),
            resolution_policy,
        })?;

        Ok(ProviderAddonUpdatePolicy {
            effective_source: applied.source,
            resolution_policy: applied.resolution_policy,
            install_dependencies: entry
                .and_then(|value| value.install_dependencies)
                .unwrap_or(false),
        })
    }

    pub(crate) fn is_ignored(&self, package: &TrackedAddonPackage) -> bool {
        self.entry_for_package(package)
            .and_then(|entry| entry.ignored)
            .unwrap_or(false)
    }

    pub(crate) fn index_update_policy(
        &self,
        package: &TrackedAddonPackage,
    ) -> IndexedAddonUpdatePolicy {
        let entry = self.entry_for_package(package);
        IndexedAddonUpdatePolicy {
            ignored: entry.and_then(|value| value.ignored).unwrap_or(false),
            install_dependencies: entry
                .and_then(|value| value.install_dependencies)
                .unwrap_or(false),
        }
    }

    fn entry_for_package(&self, package: &TrackedAddonPackage) -> Option<&AddonPolicyPackageEntry> {
        self.entries
            .get(&normalize_package_key(&package.package_id))
    }
}

fn provider_resolution_policy(
    entry: Option<&AddonPolicyPackageEntry>,
) -> AddonSourceResolutionPolicy {
    let Some(entry) = entry else {
        return AddonSourceResolutionPolicy::default();
    };

    AddonSourceResolutionPolicy {
        release_channel: entry.release_channel,
        allow_prerelease: entry.allow_prerelease,
    }
}

fn normalize_package_key(package_id: &str) -> String {
    package_id.trim().to_ascii_lowercase()
}
