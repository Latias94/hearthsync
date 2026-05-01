mod attach;
mod curation;
mod inspection;
mod operations;
mod package;
mod shared;

pub use attach::{
    AddonIndexAttachPackageResult, AddonIndexAttachPackageStatusResult, AddonIndexAttachResult,
};
pub use curation::{
    AddonIndexPackageSuggestionResult, AddonIndexPackageSuggestionStatusResult,
    AddonIndexScaffoldResult, AddonIndexSuggestionResult,
};
pub use inspection::{
    AddonIndexIdentityHintCoverageResult, AddonIndexInspectionResult,
    AddonIndexInspectionWarningCodeResult, AddonIndexInspectionWarningResult,
    AddonIndexInspectionWarningSeverityResult, AddonIndexValidationResult,
};
pub use operations::{AddonIndexInstallResult, AddonIndexRelinkResult, AddonIndexUpdateResult};
pub use package::AddonIndexPackageResult;
pub use shared::AddonIndexTrackedMatchStrategyResult;
