use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::{AddonInventory, AddonProvider};

use super::super::super::map_owned_vec;
use super::tracked::TrackedAddonPackageResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonInventoryResult {
    pub target_addon_root: PathBuf,
    pub registry_path: PathBuf,
    pub tracked_package_count: usize,
    pub tracked_addon_count: usize,
    pub tracked_packages: Vec<TrackedAddonPackageResult>,
    pub untracked_addons: Vec<String>,
}

impl AddonInventoryResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonInventory, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let tracked_package_count = value.tracked_packages.len();
        let tracked_addon_count = value
            .tracked_packages
            .iter()
            .map(|package| package.addons.len())
            .sum();

        Self {
            target_addon_root: value.target_addon_root,
            registry_path: value.registry_path,
            tracked_package_count,
            tracked_addon_count,
            tracked_packages: map_owned_vec(value.tracked_packages, |value| {
                TrackedAddonPackageResult::from_domain_with_provider(value, provider)
            }),
            untracked_addons: value.untracked_addons,
        }
    }
}
