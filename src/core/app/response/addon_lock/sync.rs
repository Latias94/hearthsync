use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::lock::{
    AddonLockApplyResult as DomainAddonLockApplyResult,
    AddonLockPlanResult as DomainAddonLockPlanResult,
    AddonLockSyncAction as DomainAddonLockSyncAction,
    AddonLockSyncActionKind as DomainAddonLockSyncActionKind,
};

use super::super::super::map_owned_vec;
use super::super::addon::AddonSourceResult;
use super::verify::AddonLockVerifyResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonLockSyncActionKindResult {
    Install,
    Update,
    Remove,
    MetadataOnly,
}

impl AddonLockSyncActionKindResult {
    fn from_domain(value: DomainAddonLockSyncActionKind) -> Self {
        match value {
            DomainAddonLockSyncActionKind::Install => Self::Install,
            DomainAddonLockSyncActionKind::Update => Self::Update,
            DomainAddonLockSyncActionKind::Remove => Self::Remove,
            DomainAddonLockSyncActionKind::MetadataOnly => Self::MetadataOnly,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockSyncActionResult {
    pub kind: AddonLockSyncActionKindResult,
    pub comparison_key: String,
    pub package_id: String,
    pub name: Option<String>,
    pub addon_directories: Vec<String>,
    pub source: Option<AddonSourceResult>,
    pub source_label: Option<String>,
    pub reasons: Vec<String>,
    pub blocked_reasons: Vec<String>,
    pub requires_replace_existing: bool,
}

impl AddonLockSyncActionResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonLockSyncAction,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = value
            .source
            .map(|value| AddonSourceResult::from_domain_with_provider(value, provider));
        let source_label = source.as_ref().map(|source| source.display_name.clone());

        Self {
            kind: AddonLockSyncActionKindResult::from_domain(value.kind),
            comparison_key: value.comparison_key,
            package_id: value.package_id,
            name: value.name,
            addon_directories: value.addon_directories,
            source,
            source_label,
            reasons: value.reasons,
            blocked_reasons: value.blocked_reasons,
            requires_replace_existing: value.requires_replace_existing,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockPlanResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub action_count: usize,
    pub actions: Vec<AddonLockSyncActionResult>,
}

impl AddonLockPlanResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonLockPlanResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let untracked_addon_count = value.untracked_addons.len();
        let action_count = value.actions.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            install_count: value.install_count,
            update_count: value.update_count,
            remove_count: value.remove_count,
            metadata_only_count: value.metadata_only_count,
            unchanged_count: value.unchanged_count,
            blocked_count: value.blocked_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            action_count,
            actions: map_owned_vec(value.actions, |value| {
                AddonLockSyncActionResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockApplyResult {
    pub lock_path: PathBuf,
    pub installation_root: PathBuf,
    pub backup_path: Option<PathBuf>,
    pub install_count: usize,
    pub update_count: usize,
    pub remove_count: usize,
    pub metadata_only_count: usize,
    pub unchanged_count: usize,
    pub blocked_count: usize,
    pub untracked_addon_count: usize,
    pub untracked_addons: Vec<String>,
    pub action_count: usize,
    pub actions: Vec<AddonLockSyncActionResult>,
    pub verification: AddonLockVerifyResult,
}

impl AddonLockApplyResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonLockApplyResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let untracked_addon_count = value.untracked_addons.len();
        let action_count = value.actions.len();

        Self {
            lock_path: value.lock_path,
            installation_root: value.installation_root,
            backup_path: value.backup_path,
            install_count: value.install_count,
            update_count: value.update_count,
            remove_count: value.remove_count,
            metadata_only_count: value.metadata_only_count,
            unchanged_count: value.unchanged_count,
            blocked_count: value.blocked_count,
            untracked_addon_count,
            untracked_addons: value.untracked_addons,
            action_count,
            actions: map_owned_vec(value.actions, |value| {
                AddonLockSyncActionResult::from_domain_with_provider(value, provider)
            }),
            verification: AddonLockVerifyResult::from_domain_with_provider(
                value.verification,
                provider,
            ),
        }
    }
}
