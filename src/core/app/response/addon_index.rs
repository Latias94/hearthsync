use std::path::PathBuf;

use serde::Serialize;

use crate::core::addon::AddonProvider;
use crate::core::addon::index::{
    AddonIndexAttachPackageResult as DomainAddonIndexAttachPackageResult,
    AddonIndexAttachPackageStatus as DomainAddonIndexAttachPackageStatus,
    AddonIndexAttachResult as DomainAddonIndexAttachResult, AddonIndexIdentityHintCoverage,
    AddonIndexInspection, AddonIndexInspectionWarning as DomainAddonIndexInspectionWarning,
    AddonIndexInspectionWarningCode as DomainAddonIndexInspectionWarningCode,
    AddonIndexInspectionWarningSeverity as DomainAddonIndexInspectionWarningSeverity,
    AddonIndexInstallResult as DomainAddonIndexInstallResult, AddonIndexPackage,
    AddonIndexPackageSuggestion as DomainAddonIndexPackageSuggestion,
    AddonIndexPackageSuggestionStatus as DomainAddonIndexPackageSuggestionStatus,
    AddonIndexRelinkResult as DomainAddonIndexRelinkResult,
    AddonIndexScaffoldResult as DomainAddonIndexScaffoldResult,
    AddonIndexSuggestion as DomainAddonIndexSuggestion,
    AddonIndexTrackedMatchStrategy as DomainAddonIndexTrackedMatchStrategy,
    AddonIndexUpdateResult as DomainAddonIndexUpdateResult,
};

use super::super::map_owned_vec;
use super::addon::{
    AddonSourceResult, InstalledAddonPackageResult, TrackedAddonResult, UpdatedAddonPackageResult,
};
use crate::core::app::AddonPackageMetadataValue;

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexPackageResult {
    pub id: String,
    pub name: String,
    pub version: String,
    pub match_package_ids: Vec<String>,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub source_url: Option<String>,
    pub website_url: Option<String>,
    pub sha256: Option<String>,
    pub addon_directories: Vec<String>,
    pub supported_flavors: Vec<String>,
}

impl AddonIndexPackageResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonIndexPackage, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();

        Self {
            id: value.id,
            name: value.name,
            version: value.version,
            match_package_ids: value.match_package_ids,
            source,
            source_label,
            source_url: value.source_url,
            website_url: value.website_url,
            sha256: value.sha256,
            addon_directories: value.addon_directories,
            supported_flavors: value.supported_flavors,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspectionResult {
    pub index_path: PathBuf,
    pub name: String,
    pub description: Option<String>,
    pub package_count: usize,
    pub identity_hint_coverage: AddonIndexIdentityHintCoverageResult,
    pub warning_count: usize,
    pub blocking_warning_count: usize,
    pub advisory_warning_count: usize,
    pub warnings: Vec<AddonIndexInspectionWarningResult>,
    pub packages: Vec<AddonIndexPackageResult>,
}

impl AddonIndexInspectionResult {
    pub(crate) fn from_domain_with_provider<P>(value: AddonIndexInspection, provider: &P) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            index_path: value.index_path,
            name: value.index.name,
            description: value.index.description,
            package_count: value.package_count,
            identity_hint_coverage: AddonIndexIdentityHintCoverageResult::from_domain(
                value.identity_hint_coverage,
            ),
            warning_count: value.warning_count,
            blocking_warning_count: value.blocking_warning_count,
            advisory_warning_count: value.advisory_warning_count,
            warnings: map_owned_vec(
                value.warnings,
                AddonIndexInspectionWarningResult::from_domain,
            ),
            packages: map_owned_vec(value.index.packages, |value| {
                AddonIndexPackageResult::from_domain_with_provider(value, provider)
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexValidationResult {
    pub index_path: PathBuf,
    pub name: String,
    pub package_count: usize,
    pub identity_hint_coverage: AddonIndexIdentityHintCoverageResult,
    pub valid: bool,
    pub warning_count: usize,
    pub blocking_warning_count: usize,
    pub advisory_warning_count: usize,
    pub warnings: Vec<AddonIndexInspectionWarningResult>,
}

impl AddonIndexValidationResult {
    pub(crate) fn from_inspection(value: AddonIndexInspectionResult) -> Self {
        Self {
            index_path: value.index_path,
            name: value.name,
            package_count: value.package_count,
            identity_hint_coverage: value.identity_hint_coverage,
            valid: value.blocking_warning_count == 0,
            warning_count: value.warning_count,
            blocking_warning_count: value.blocking_warning_count,
            advisory_warning_count: value.advisory_warning_count,
            warnings: value.warnings,
        }
    }
}

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

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexSuggestionResult {
    pub index_path: PathBuf,
    pub index_name: String,
    pub index_package_count: usize,
    pub considered_package_count: usize,
    pub suggested_package_count: usize,
    pub complete_package_count: usize,
    pub no_match_package_count: usize,
    pub ambiguous_match_package_count: usize,
    pub skipped_unsupported_flavor_package_count: usize,
    pub packages: Vec<AddonIndexPackageSuggestionResult>,
}

impl AddonIndexSuggestionResult {
    pub(crate) fn from_domain(value: DomainAddonIndexSuggestion) -> Self {
        Self {
            index_path: value.index_path,
            index_name: value.index_name,
            index_package_count: value.index_package_count,
            considered_package_count: value.considered_package_count,
            suggested_package_count: value.suggested_package_count,
            complete_package_count: value.complete_package_count,
            no_match_package_count: value.no_match_package_count,
            ambiguous_match_package_count: value.ambiguous_match_package_count,
            skipped_unsupported_flavor_package_count: value
                .skipped_unsupported_flavor_package_count,
            packages: map_owned_vec(
                value.packages,
                AddonIndexPackageSuggestionResult::from_domain,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexScaffoldResult {
    pub index_path: PathBuf,
    pub index_name: String,
    pub package_count: usize,
    pub used_metadata_package_count: usize,
    pub inferred_name_package_count: usize,
    pub inferred_version_package_count: usize,
    pub placeholder_version_package_count: usize,
    pub package_ids: Vec<String>,
}

impl AddonIndexScaffoldResult {
    pub(crate) fn from_domain(value: DomainAddonIndexScaffoldResult) -> Self {
        Self {
            index_path: value.index_path,
            index_name: value.index_name,
            package_count: value.package_count,
            used_metadata_package_count: value.used_metadata_package_count,
            inferred_name_package_count: value.inferred_name_package_count,
            inferred_version_package_count: value.inferred_version_package_count,
            placeholder_version_package_count: value.placeholder_version_package_count,
            package_ids: value.package_ids,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexPackageSuggestionStatusResult {
    Suggested,
    Complete,
    NoLocalMatch,
    AmbiguousLocalMatch,
}

impl AddonIndexPackageSuggestionStatusResult {
    fn from_domain(value: DomainAddonIndexPackageSuggestionStatus) -> Self {
        match value {
            DomainAddonIndexPackageSuggestionStatus::Suggested => Self::Suggested,
            DomainAddonIndexPackageSuggestionStatus::Complete => Self::Complete,
            DomainAddonIndexPackageSuggestionStatus::NoLocalMatch => Self::NoLocalMatch,
            DomainAddonIndexPackageSuggestionStatus::AmbiguousLocalMatch => {
                Self::AmbiguousLocalMatch
            }
        }
    }
}

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
    fn from_domain(value: DomainAddonIndexTrackedMatchStrategy) -> Self {
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

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexPackageSuggestionResult {
    pub package_id: String,
    pub package_name: String,
    pub current_match_package_ids: Vec<String>,
    pub current_addon_directories: Vec<String>,
    pub status: AddonIndexPackageSuggestionStatusResult,
    pub matched_tracked_package_id: Option<String>,
    pub match_strategy: Option<AddonIndexTrackedMatchStrategyResult>,
    pub matched_addon_directories: Vec<String>,
    pub match_package_ids_to_add: Vec<String>,
    pub addon_directories_to_add: Vec<String>,
    pub message: String,
}

impl AddonIndexPackageSuggestionResult {
    fn from_domain(value: DomainAddonIndexPackageSuggestion) -> Self {
        Self {
            package_id: value.package_id,
            package_name: value.package_name,
            current_match_package_ids: value.current_match_package_ids,
            current_addon_directories: value.current_addon_directories,
            status: AddonIndexPackageSuggestionStatusResult::from_domain(value.status),
            matched_tracked_package_id: value.matched_tracked_package_id,
            match_strategy: value
                .match_strategy
                .map(AddonIndexTrackedMatchStrategyResult::from_domain),
            matched_addon_directories: value.matched_addon_directories,
            match_package_ids_to_add: value.match_package_ids_to_add,
            addon_directories_to_add: value.addon_directories_to_add,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexInspectionWarningCodeResult {
    MissingMatchPackageIds,
    MissingAddonDirectories,
    MissingExactIdentityHints,
}

impl AddonIndexInspectionWarningCodeResult {
    fn from_domain(value: DomainAddonIndexInspectionWarningCode) -> Self {
        match value {
            DomainAddonIndexInspectionWarningCode::MissingMatchPackageIds => {
                Self::MissingMatchPackageIds
            }
            DomainAddonIndexInspectionWarningCode::MissingAddonDirectories => {
                Self::MissingAddonDirectories
            }
            DomainAddonIndexInspectionWarningCode::MissingExactIdentityHints => {
                Self::MissingExactIdentityHints
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonIndexInspectionWarningSeverityResult {
    Blocking,
    Advisory,
}

impl AddonIndexInspectionWarningSeverityResult {
    fn from_domain(value: DomainAddonIndexInspectionWarningSeverity) -> Self {
        match value {
            DomainAddonIndexInspectionWarningSeverity::Blocking => Self::Blocking,
            DomainAddonIndexInspectionWarningSeverity::Advisory => Self::Advisory,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInspectionWarningResult {
    pub code: AddonIndexInspectionWarningCodeResult,
    pub severity: AddonIndexInspectionWarningSeverityResult,
    pub package_id: String,
    pub message: String,
}

impl AddonIndexInspectionWarningResult {
    fn from_domain(value: DomainAddonIndexInspectionWarning) -> Self {
        Self {
            code: AddonIndexInspectionWarningCodeResult::from_domain(value.code),
            severity: AddonIndexInspectionWarningSeverityResult::from_domain(value.severity),
            package_id: value.package_id,
            message: value.message,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexIdentityHintCoverageResult {
    pub package_count_with_both_exact_hints: usize,
    pub package_count_with_any_exact_hints: usize,
    pub package_count_with_match_package_ids: usize,
    pub package_count_with_addon_directories: usize,
    pub package_count_without_match_package_ids: usize,
    pub package_count_without_addon_directories: usize,
    pub package_count_without_exact_hints: usize,
    pub packages_without_match_package_ids: Vec<String>,
    pub packages_without_addon_directories: Vec<String>,
    pub packages_without_exact_hints: Vec<String>,
}

impl AddonIndexIdentityHintCoverageResult {
    fn from_domain(value: AddonIndexIdentityHintCoverage) -> Self {
        Self {
            package_count_with_both_exact_hints: value.package_count_with_both_exact_hints,
            package_count_with_any_exact_hints: value.package_count_with_any_exact_hints,
            package_count_with_match_package_ids: value.package_count_with_match_package_ids,
            package_count_with_addon_directories: value.package_count_with_addon_directories,
            package_count_without_match_package_ids: value.package_count_without_match_package_ids,
            package_count_without_addon_directories: value.package_count_without_addon_directories,
            package_count_without_exact_hints: value.package_count_without_exact_hints,
            packages_without_match_package_ids: value.packages_without_match_package_ids,
            packages_without_addon_directories: value.packages_without_addon_directories,
            packages_without_exact_hints: value.packages_without_exact_hints,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexInstallResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackageResult,
    pub install: InstalledAddonPackageResult,
}

impl AddonIndexInstallResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonIndexInstallResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        Self {
            index_path: value.index_path,
            package: AddonIndexPackageResult::from_domain_with_provider(value.package, provider),
            install: InstalledAddonPackageResult::from_domain_with_provider(
                value.install,
                provider,
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexRelinkResult {
    pub index_path: PathBuf,
    pub package: AddonIndexPackageResult,
    pub dry_run: bool,
    pub tracked_package_id: String,
    pub previous_source: AddonSourceResult,
    pub previous_source_label: String,
    pub source: AddonSourceResult,
    pub source_label: String,
    pub addon_count: usize,
    pub addons: Vec<TrackedAddonResult>,
    pub metadata: AddonPackageMetadataValue,
    pub registry_path: PathBuf,
    pub source_changed: bool,
    pub metadata_changed: bool,
}

impl AddonIndexRelinkResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonIndexRelinkResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let previous_source =
            AddonSourceResult::from_domain_with_provider(value.previous_source, provider);
        let previous_source_label = previous_source.display_name.clone();
        let source = AddonSourceResult::from_domain_with_provider(value.source, provider);
        let source_label = source.display_name.clone();
        let addon_count = value.addons.len();

        Self {
            index_path: value.index_path,
            package: AddonIndexPackageResult::from_domain_with_provider(value.package, provider),
            dry_run: value.dry_run,
            tracked_package_id: value.tracked_package_id,
            previous_source,
            previous_source_label,
            source,
            source_label,
            addon_count,
            addons: map_owned_vec(value.addons, TrackedAddonResult::from_domain),
            metadata: AddonPackageMetadataValue::from_domain(value.metadata),
            registry_path: value.registry_path,
            source_changed: value.source_changed,
            metadata_changed: value.metadata_changed,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AddonIndexUpdateResult {
    pub index_path: PathBuf,
    pub selected_package_count: usize,
    pub selected_packages: Vec<AddonIndexPackageResult>,
    pub update: UpdatedAddonPackageResult,
}

impl AddonIndexUpdateResult {
    pub(crate) fn from_domain_with_provider<P>(
        value: DomainAddonIndexUpdateResult,
        provider: &P,
    ) -> Self
    where
        P: AddonProvider + ?Sized,
    {
        let selected_package_count = value.selected_packages.len();

        Self {
            index_path: value.index_path,
            selected_package_count,
            selected_packages: map_owned_vec(value.selected_packages, |value| {
                AddonIndexPackageResult::from_domain_with_provider(value, provider)
            }),
            update: UpdatedAddonPackageResult::from_domain_with_provider(value.update, provider),
        }
    }
}
