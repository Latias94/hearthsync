mod addon_lock;
mod addon_source_archive;
mod apply;
mod apply_model;
mod apply_policy;
mod apply_source;
mod archive_read;
mod character_mapping;
mod character_mapping_match;
mod constants;
mod entry_layout;
mod entry_plan;
mod execution;
mod external_package;
mod packing;
mod planner;
mod shared;
mod target_accounts;
#[cfg(test)]
mod tests;
mod types;
mod wtf_archive;
mod wtf_scope;
mod zip_write;

pub use addon_lock::{apply_bundle_addon_lock, plan_bundle_addon_lock};
pub use apply::{unpack_bundle, unpack_bundle_task};
pub use external_package::analyze::analyze_external_package;
pub use external_package::create_bundle::create_external_package_bundle;
pub(crate) use external_package::manifest::author_package_apply_defaults;
pub use external_package::plan::plan_external_package_apply;
pub use external_package::tasks::{
    analyze_external_package_task, apply_external_package, apply_external_package_task,
    plan_external_package_apply_task,
};
pub use external_package::types::{
    AnalyzeExternalPackageRequest, AppliedExternalPackage, ApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest, ExternalPackageAnalysis, ExternalPackageApplyPlan,
    ExternalPackageEntry, ExternalPackageLayout, ExternalPackageSourceCharacterSummary,
    ExternalPackageSourceIdentitySummary, ExternalPackageSourceKind, ExternalPackageSummary,
    ExternalPackageWarning, ExternalPackageWarningCategory, ExternalPackageWarningCode,
    ExternalPackageWarningGroup, ExternalPackageWtfScopeSummary, PlanExternalPackageApplyRequest,
    PreparedExternalPackageBundle,
};
pub use packing::inspect::{inspect_bundle, load_apply_mappings};
pub use packing::pack::pack_bundle;
pub use planner::pipeline::plan_bundle_apply;
pub use types::addon_lock::{
    BundleAddonLockApply, BundleAddonLockApplyRequest, BundleAddonLockPlan,
};
pub use types::apply::{
    ApplyAction, ApplyGroup, ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary,
    BundleApplyMappings, BundleApplyPlan, CharacterMappingOverride, GroupPolicy,
    UnpackBundleRequest, UnpackedBundle, WtfScope, WtfScopeRisk,
};
pub use types::archive::{BundleEntryCounts, BundleInspection, CreatedBundle, PackBundleRequest};
