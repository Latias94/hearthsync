use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::index::{
    AddonIndexAttachPackageResult as DomainAddonIndexAttachPackageResult,
    AddonIndexAttachPackageStatus as DomainAddonIndexAttachPackageStatus,
    AddonIndexAttachResult as DomainAddonIndexAttachResult,
};

use super::super::super::map_owned_vec;
use super::super::addon::AddonSourceResult;
use super::package::AddonIndexPackageResult;
use super::shared::AddonIndexTrackedMatchStrategyResult;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexAttachPackageStatusResult {
    WouldAttach,
    Attached,
    AlreadyAttached,
    NoLocalMatch,
    AmbiguousLocalMatch,
    AddonDirectoryMismatch,
    PrepareFailed,
    SkippedUnsupportedFlavor,
}

impl AddonIndexAttachPackageStatusResult {
    fn from_domain(value: DomainAddonIndexAttachPackageStatus) -> Self {
        match value {
            DomainAddonIndexAttachPackageStatus::WouldAttach => Self::WouldAttach,
            DomainAddonIndexAttachPackageStatus::Attached => Self::Attached,
            DomainAddonIndexAttachPackageStatus::AlreadyAttached => Self::AlreadyAttached,
            DomainAddonIndexAttachPackageStatus::NoLocalMatch => Self::NoLocalMatch,
            DomainAddonIndexAttachPackageStatus::AmbiguousLocalMatch => Self::AmbiguousLocalMatch,
            DomainAddonIndexAttachPackageStatus::AddonDirectoryMismatch => {
                Self::AddonDirectoryMismatch
            }
            DomainAddonIndexAttachPackageStatus::PrepareFailed => Self::PrepareFailed,
            DomainAddonIndexAttachPackageStatus::SkippedUnsupportedFlavor => {
                Self::SkippedUnsupportedFlavor
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexAttachPackageResult {
    pub package: AddonIndexPackageResult,
    pub status: AddonIndexAttachPackageStatusResult,
    pub matched_tracked_package_id: Option<String>,
    pub match_strategy: Option<AddonIndexTrackedMatchStrategyResult>,
    pub previous_source: Option<AddonSourceResult>,
    pub previous_source_label: Option<String>,
    pub source: Option<AddonSourceResult>,
    pub source_label: Option<String>,
    pub source_changed: bool,
    pub metadata_changed: bool,
    pub message: String,
}

impl AddonIndexAttachPackageResult {
    fn from_domain_with_provider<P>(
        value: DomainAddonIndexAttachPackageResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let previous_source = value
            .previous_source
            .map(|source| AddonSourceResult::from_domain_with_provider(source, provider));
        let previous_source_label = previous_source
            .as_ref()
            .map(|source| source.display_name.clone());
        let source = value
            .source
            .map(|source| AddonSourceResult::from_domain_with_provider(source, provider));
        let source_label = source.as_ref().map(|source| source.display_name.clone());

        Self {
            package: AddonIndexPackageResult::from_domain_with_provider(value.package, provider),
            status: AddonIndexAttachPackageStatusResult::from_domain(value.status),
            matched_tracked_package_id: value.matched_tracked_package_id,
            match_strategy: value
                .match_strategy
                .map(AddonIndexTrackedMatchStrategyResult::from_domain),
            previous_source,
            previous_source_label,
            source,
            source_label,
            source_changed: value.source_changed,
            metadata_changed: value.metadata_changed,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexAttachResult {
    pub index_path: PathBuf,
    pub index_name: String,
    pub dry_run: bool,
    pub ready: bool,
    pub applied: bool,
    pub partial_apply: bool,
    pub registry_path: PathBuf,
    pub index_package_count: usize,
    pub considered_package_count: usize,
    pub change_package_count: usize,
    pub attached_package_count: usize,
    pub already_attached_package_count: usize,
    pub blocked_package_count: usize,
    pub skipped_unsupported_flavor_package_count: usize,
    pub packages: Vec<AddonIndexAttachPackageResult>,
}

impl AddonIndexAttachResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonIndexAttachResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            index_path: value.index_path,
            index_name: value.index_name,
            dry_run: value.dry_run,
            ready: value.ready,
            applied: value.applied,
            partial_apply: value.partial_apply,
            registry_path: value.registry_path,
            index_package_count: value.index_package_count,
            considered_package_count: value.considered_package_count,
            change_package_count: value.change_package_count,
            attached_package_count: value.attached_package_count,
            already_attached_package_count: value.already_attached_package_count,
            blocked_package_count: value.blocked_package_count,
            skipped_unsupported_flavor_package_count: value
                .skipped_unsupported_flavor_package_count,
            packages: map_owned_vec(value.packages, |value| {
                AddonIndexAttachPackageResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}
