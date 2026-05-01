use serde::Serialize;

use crate::core::addon::index::AddonIndexTrackedMatchStrategy as DomainAddonIndexTrackedMatchStrategy;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexTrackedMatchStrategyResult {
    StoredIndexPackageId,
    ExactPackageId,
    CuratedMatchPackageId,
    SourceIdentity,
    SourceFamilyIdentity,
    DisplayName,
    AddonDirectories,
    AddonDirectoryOverlap,
}

impl AddonIndexTrackedMatchStrategyResult {
    pub(super) fn from_domain(value: DomainAddonIndexTrackedMatchStrategy) -> Self {
        match value {
            DomainAddonIndexTrackedMatchStrategy::StoredIndexPackageId => {
                Self::StoredIndexPackageId
            }
            DomainAddonIndexTrackedMatchStrategy::ExactPackageId => Self::ExactPackageId,
            DomainAddonIndexTrackedMatchStrategy::CuratedMatchPackageId => {
                Self::CuratedMatchPackageId
            }
            DomainAddonIndexTrackedMatchStrategy::SourceIdentity => Self::SourceIdentity,
            DomainAddonIndexTrackedMatchStrategy::SourceFamilyIdentity => {
                Self::SourceFamilyIdentity
            }
            DomainAddonIndexTrackedMatchStrategy::DisplayName => Self::DisplayName,
            DomainAddonIndexTrackedMatchStrategy::AddonDirectories => Self::AddonDirectories,
            DomainAddonIndexTrackedMatchStrategy::AddonDirectoryOverlap => {
                Self::AddonDirectoryOverlap
            }
        }
    }
}
