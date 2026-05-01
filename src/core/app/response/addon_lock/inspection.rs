use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::lock::AddonLockInspection;

use super::super::super::map_owned_vec;
use super::package::AddonLockPackageResult;

#[derive(Debug, Clone, Serialize)]
pub struct AddonLockInspectionResult {
    pub lock_path: PathBuf,
    pub generated_at: String,
    pub package_count: usize,
    pub packages: Vec<AddonLockPackageResult>,
}

impl AddonLockInspectionResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonLockInspection, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            lock_path: value.lock_path,
            generated_at: value.lock.generated_at,
            package_count: value.package_count,
            packages: map_owned_vec(value.lock.packages, |value| {
                AddonLockPackageResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}
