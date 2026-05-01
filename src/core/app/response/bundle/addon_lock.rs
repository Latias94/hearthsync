use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::bundle::{
    BundleAddonLockApply as DomainBundleAddonLockApply,
    BundleAddonLockPlan as DomainBundleAddonLockPlan,
};

use super::super::addon_lock::{AddonLockApplyResult, AddonLockPlanResult};

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockPlanResult {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub plan: AddonLockPlanResult,
}

impl BundleAddonLockPlanResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainBundleAddonLockPlan,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            bundle_path: value.bundle_path,
            embedded_lock_entry: value.embedded_lock_entry,
            plan: AddonLockPlanResult::from_domain_with_provider(value.plan, provider),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BundleAddonLockApplyResult {
    pub bundle_path: PathBuf,
    pub embedded_lock_entry: String,
    pub apply: AddonLockApplyResult,
}

impl BundleAddonLockApplyResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainBundleAddonLockApply,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            bundle_path: value.bundle_path,
            embedded_lock_entry: value.embedded_lock_entry,
            apply: AddonLockApplyResult::from_domain_with_provider(value.apply, provider),
        }
    }
}
