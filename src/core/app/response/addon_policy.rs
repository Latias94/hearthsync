use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::policy::{
    AddonPolicyInspection as DomainAddonPolicyInspection,
    AddonPolicyMutationResult as DomainAddonPolicyMutationResult,
    AddonPolicyPackageView as DomainAddonPolicyPackageView,
};
use crate::core::app::{AddonPolicyPinValue, AddonReleaseChannelValue};

#[derive(Debug, Clone, Serialize)]
pub struct AddonPolicyPackageResult {
    pub package_id: String,
    pub package_name: Option<String>,
    pub addon_directories: Vec<String>,
    pub tracked: bool,
    pub ignored: Option<bool>,
    pub pin: Option<AddonPolicyPinValue>,
    pub release_channel: Option<AddonReleaseChannelValue>,
    pub allow_prerelease: Option<bool>,
    pub install_dependencies: Option<bool>,
}

impl AddonPolicyPackageResult {
    pub(crate) fn from_domain(value: DomainAddonPolicyPackageView) -> Self {
        Self {
            package_id: value.package_id,
            package_name: value.package_name,
            addon_directories: value.addon_directories,
            tracked: value.tracked,
            ignored: value.ignored,
            pin: value.pin.map(AddonPolicyPinValue::from_domain),
            release_channel: value
                .release_channel
                .map(AddonReleaseChannelValue::from_domain),
            allow_prerelease: value.allow_prerelease,
            install_dependencies: value.install_dependencies,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonPolicyInspectionResult {
    pub policy_path: PathBuf,
    pub package_count: usize,
    pub packages: Vec<AddonPolicyPackageResult>,
}

impl AddonPolicyInspectionResult {
    pub(crate) fn from_domain(value: DomainAddonPolicyInspection) -> Self {
        Self {
            policy_path: value.policy_path,
            package_count: value.package_count,
            packages: value
                .packages
                .into_iter()
                .map(AddonPolicyPackageResult::from_domain)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonPolicyMutationResult {
    pub policy_path: PathBuf,
    pub package_count: usize,
    pub package_id: String,
    pub entry_removed: bool,
    pub package: Option<AddonPolicyPackageResult>,
}

impl AddonPolicyMutationResult {
    pub(crate) fn from_domain(value: DomainAddonPolicyMutationResult) -> Self {
        Self {
            policy_path: value.policy_path,
            package_count: value.package_count,
            package_id: value.package_id,
            entry_removed: value.entry_removed,
            package: value.package.map(AddonPolicyPackageResult::from_domain),
        }
    }
}
