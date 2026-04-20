use crate::core::addon::TrackedAddonPackage;

use super::{AddonLockPackage, AddonLockPlanResult, AddonLockSyncAction};

#[derive(Debug, Clone)]
pub(super) struct PlannedLockAction {
    pub(super) action: AddonLockSyncAction,
    pub(super) expected: Option<AddonLockPackage>,
    pub(super) current: Option<TrackedAddonPackage>,
}

#[derive(Debug, Clone)]
pub(super) struct AddonLockPlanContext {
    pub(super) result: AddonLockPlanResult,
    pub(super) actions: Vec<PlannedLockAction>,
}
