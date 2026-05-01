use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::lock::AddonLockWriteResult as DomainAddonLockWriteResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockWriteResult {
    pub lock_path: PathBuf,
    pub package_count: usize,
    pub removed: bool,
}

impl AddonLockWriteResult {
    pub(crate) fn from_domain(value: DomainAddonLockWriteResult) -> Self {
        Self {
            lock_path: value.lock_path,
            package_count: value.package_count,
            removed: value.removed,
        }
    }
}
